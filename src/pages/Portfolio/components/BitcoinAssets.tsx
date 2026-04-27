import React, { useState, useEffect } from 'react';
import { MnemonicBackupDialog } from '@/components/common/MnemonicBackupDialog';
import RecoveryPanel from '@/components/common/RecoveryPanel';
import { useSecuritySession } from '@/components/common/SecuritySession';
import { UnlockGate } from '@/components/common/UnlockGate';
import { Card, Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger, Tabs, TabsContent, TabsList, TabsTrigger, Label, Textarea, Input, Badge } from '@/components/ui';
import { Copy, Plus, AlertCircle, CheckCircle2, Trash2, Download, RefreshCw, Send, ExternalLink, HelpCircle } from 'lucide-react';
import { invoke, isTauriRuntimeAvailable, TAURI_UNAVAILABLE_MESSAGE, isTauriUnavailableError } from '@/lib/tauri';
import { parseSecurityError, securityGetBackendState } from '@/lib/security';
import { formatFreshnessLabel, getFreshnessBadgeClass, FreshnessMetadata } from '@/lib/evm-wallet';
import { describeWalletRecovery, type WalletRecoveryGuidance } from '@/lib/wallet-recovery';
import { shortAddress, getBitcoinExplorerUrl, openExternalLink } from '@/lib/utils';
import { toast } from 'sonner';

interface WalletInfo {
  id: string;
  label: string;
  wallet_type: 'mnemonic' | 'private-key';
  address: string;
  balance: number;
  created_at: string;
  updated_at: string;
}

interface CreateWalletResponse {
  revealed_secret: string | null;
  revealed_secret_type: 'mnemonic' | 'private-key' | null;
  wallet: WalletInfo;
}

interface PendingMnemonicBackup {
  mnemonic: string;
  walletLabel: string;
}

interface PriceState {
  price_usd: number | null;
  price_source: string | null;
  price_updated_at: number | null;
  status: 'fresh' | 'cached' | 'stale' | 'partial' | 'unavailable' | 'synthetic';
}

interface BalanceState {
  raw_amount: string;
  display_amount: number;
  chain_id: string | null;
  freshness: FreshnessMetadata;
}

interface BitcoinWalletBalanceResponse {
  wallet: WalletInfo;
  balance_state: BalanceState;
}

interface ReviewedBitcoinSendIntent {
  actionLabel: string;
  realChainAction: string;
  walletLabel: string;
  fromAddress: string;
  recipientAddress: string;
  amountDisplay: string;
  amountSats: string;
  feeRateDisplay: string;
  estimatedFeeDisplay: string;
  totalChargeDisplay: string;
  sendAll: boolean;
  riskPoint: string;
  request: {
    wallet_id: string;
    to_address: string;
    amount: number;
    fee_rate: number;
    send_all: boolean;
  };
  payloadFingerprint: string;
}

const SATOSHIS_PER_BTC = 100_000_000n;

const normalizeBtcAmountToSats = (amount: string) => {
  const normalizedAmount = amount.trim();
  if (!normalizedAmount) {
    throw new Error('Enter a valid BTC amount');
  }

  if (!/^(?:\d+\.\d+|\d+|\.\d+)$/.test(normalizedAmount)) {
    throw new Error('Enter a valid BTC amount');
  }

  const [integerPartRaw, fractionalPartRaw = ''] = normalizedAmount.split('.');
  const integerPart = integerPartRaw === '' ? '0' : integerPartRaw;
  const significantFractionLength = fractionalPartRaw.replace(/0+$/, '').length;
  if (significantFractionLength > 8) {
    throw new Error('BTC amount supports up to 8 decimal places');
  }

  const fractionalPart = fractionalPartRaw.padEnd(8, '0').slice(0, 8);
  const satoshiValue = `${integerPart}${fractionalPart}`.replace(/^0+(?=\d)/, '');
  if (!/^\d+$/.test(satoshiValue) || /^0+$/.test(satoshiValue)) {
    throw new Error('Enter a valid BTC amount');
  }

  return satoshiValue;
};

const formatBtcFromSats = (satoshis: string) => {
  const satoshiValue = BigInt(satoshis);
  const wholePart = satoshiValue / SATOSHIS_PER_BTC;
  const fractionalPart = (satoshiValue % SATOSHIS_PER_BTC).toString().padStart(8, '0');
  return `${wholePart.toString()}.${fractionalPart}`;
};

const estimateBtcFeeSats = (feeRate: number) => Math.max(1, Math.trunc(feeRate)) * 148;

const buildBitcoinPayloadFingerprint = (request: ReviewedBitcoinSendIntent['request']) => [
  request.wallet_id,
  request.to_address,
  request.amount.toString(),
  request.fee_rate.toString(),
  request.send_all ? '1' : '0',
].join('|');

const BitcoinAssets: React.FC = () => {
  const { requestUnlock } = useSecuritySession();
  const [wallets, setWallets] = useState<WalletInfo[]>([]);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [pendingMnemonicBackup, setPendingMnemonicBackup] = useState<PendingMnemonicBackup | null>(null);
  const [mnemonicInput, setMnemonicInput] = useState('');
  const [privateKeyInput, setPrivateKeyInput] = useState('');
  const [walletLabel, setWalletLabel] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isPersistingMnemonic, setIsPersistingMnemonic] = useState(false);
  const [mnemonicCopied, setMnemonicCopied] = useState(false);
  const [exportedSecret, setExportedSecret] = useState<string | null>(null);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [exportedSecretType, setExportedSecretType] = useState<'mnemonic' | 'private-key'>('private-key');
  const [exportCopied, setExportCopied] = useState(false);
  const [addressCopied, setAddressCopied] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [refreshingBalance, setRefreshingBalance] = useState<string | null>(null);
  const [btcPrice, setBtcPrice] = useState<PriceState>({
    price_usd: null,
    price_source: null,
    price_updated_at: null,
    status: 'unavailable',
  });
  const [walletBalanceStates, setWalletBalanceStates] = useState<Map<string, BalanceState>>(new Map());

  // Send BTC State
  const [isSendDialogOpen, setIsSendDialogOpen] = useState(false);
  const [selectedWalletForSend, setSelectedWalletForSend] = useState<WalletInfo | null>(null);
  const [sendToAddress, setSendToAddress] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [sendFeeRate, setSendFeeRate] = useState<number>(1);
  const [isSending, setIsSending] = useState(false);
  const [pendingSendReview, setPendingSendReview] = useState<ReviewedBitcoinSendIntent | null>(null);

  // Fee Estimation State
  const [isEstimatingFees, setIsEstimatingFees] = useState(false);
  const [estimatedFees, setEstimatedFees] = useState<{ fast: number; half_hour: number; hour: number } | null>(null);
  const [feeRateType, setFeeRateType] = useState<'fast' | 'half_hour' | 'hour' | 'custom'>('half_hour');
  const [isSendAll, setIsSendAll] = useState(false);
  const [walletFlowRecovery, setWalletFlowRecovery] = useState<WalletRecoveryGuidance | null>(null);
  const [sendFlowRecovery, setSendFlowRecovery] = useState<WalletRecoveryGuidance | null>(null);

  // Load wallets on mount
  useEffect(() => {
    loadWallets();
    fetchBtcPrice();

    // Refresh BTC price every 60 seconds
    const priceInterval = setInterval(fetchBtcPrice, 60000);

    return () => clearInterval(priceInterval);
  }, []);

  const loadWallets = async () => {
    if (!isTauriRuntimeAvailable()) {
      setWallets([]);
      setWalletBalanceStates(new Map());
      return;
    }

    try {
      const result = await invoke<WalletInfo[]>('bitcoin_get_wallets');
      const enrichedWallets: WalletInfo[] = [];
      const states = new Map<string, BalanceState>();
      for (const wallet of result) {
        try {
          const response = await invoke<BitcoinWalletBalanceResponse>('query_bitcoin_wallet_balance', { walletId: wallet.id });
          enrichedWallets.push(response.wallet);
          states.set(wallet.id, response.balance_state);
        } catch (error) {
          console.error(`Error loading wallet freshness for ${wallet.id}:`, error);
          enrichedWallets.push(wallet);
        }
      }
      setWallets(enrichedWallets);
      setWalletBalanceStates(states);

      if (enrichedWallets.length > 0) {
        void refreshWalletsOnStartup(enrichedWallets);
      }
    } catch (error) {
      console.error('Error loading wallets:', error);
    }
  };

  const refreshWalletsOnStartup = async (walletsToRefresh: WalletInfo[]) => {
    for (const wallet of walletsToRefresh) {
      await handleRefreshBalance(wallet.id, { silent: true, background: true });
    }
  };

  const fetchFeeEstimates = async () => {
    if (!isTauriRuntimeAvailable()) {
      setEstimatedFees(null);
      return;
    }

    setIsEstimatingFees(true);
    try {
      const fees = await invoke<{ fast: number; half_hour: number; hour: number }>('bitcoin_estimate_fees');
      setEstimatedFees(fees);
      if (feeRateType === 'fast') setSendFeeRate(Math.ceil(fees.fast));
      else if (feeRateType === 'half_hour') setSendFeeRate(Math.ceil(fees.half_hour));
      else if (feeRateType === 'hour') setSendFeeRate(Math.ceil(fees.hour));
    } catch (error) {
      console.error('Error fetching fee estimates:', error);
    } finally {
      setIsEstimatingFees(false);
    }
  };

  // Effect to fetch fees when dialog opens
  useEffect(() => {
    if (isSendDialogOpen) {
      fetchFeeEstimates();
    }
  }, [isSendDialogOpen]);

  // Effect to update sendFeeRate when feeRateType changes
  useEffect(() => {
    if (!estimatedFees) return;
    if (feeRateType === 'fast') setSendFeeRate(Math.ceil(estimatedFees.fast));
    else if (feeRateType === 'half_hour') setSendFeeRate(Math.ceil(estimatedFees.half_hour));
    else if (feeRateType === 'hour') setSendFeeRate(Math.ceil(estimatedFees.hour));
  }, [feeRateType, estimatedFees]);

  const truncateBtc = (val: number | undefined) => {
    if (val === undefined) return "0.00000000";
    return (Math.floor(val * 100000000) / 100000000).toFixed(8);
  };

  const fetchBtcPrice = async () => {
    if (!isTauriRuntimeAvailable()) {
      setBtcPrice({
        price_usd: null,
        price_source: null,
        price_updated_at: null,
        status: 'unavailable',
      });
      return;
    }

    try {
      const price = await invoke<PriceState>('state_get_bitcoin_price_state');
      setBtcPrice(price);
    } catch (error) {
      console.error('Error fetching BTC price from backend:', error);
      setBtcPrice({
        price_usd: null,
        price_source: null,
        price_updated_at: null,
        status: 'unavailable',
      });
    }
  };

  const getPriceBadgeClass = (status: PriceState['status']) => {
    switch (status) {
      case 'fresh':
        return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600';
      case 'cached':
        return 'border-slate-500/30 bg-slate-500/10 text-slate-600';
      case 'stale':
        return 'border-amber-500/30 bg-amber-500/10 text-amber-700';
      case 'partial':
        return 'border-sky-500/30 bg-sky-500/10 text-sky-700';
      case 'synthetic':
        return 'border-sky-500/30 bg-sky-500/10 text-sky-700';
      case 'unavailable':
        return 'border-red-500/30 bg-red-500/10 text-red-700';
    }
  };

  const getPriceStatusLabel = (status: PriceState['status']) => {
    switch (status) {
      case 'fresh':
        return 'Fresh';
      case 'cached':
        return 'Cached';
      case 'stale':
        return 'Stale';
      case 'partial':
        return 'Partial';
      case 'synthetic':
        return 'Synthetic';
      case 'unavailable':
        return 'Unavailable';
    }
  };

  const getPriceStatusDescription = (priceState: PriceState) => {
    const updatedLabel = formatUpdatedLabel(priceState.price_updated_at, 'Price timestamp unavailable');

    switch (priceState.status) {
      case 'fresh':
        return `${updatedLabel}. Market price is current.`;
      case 'cached':
        return `${updatedLabel}. Showing cached market price.`;
      case 'stale':
        return `${updatedLabel}. Showing stale market price until refresh succeeds.`;
      case 'partial':
        return `${updatedLabel}. Price view is partially refreshed.`;
      case 'synthetic':
        return `${updatedLabel}. This is a synthetic fallback price, not a live market quote.`;
      case 'unavailable':
        return 'BTC market price is currently unavailable.';
    }
  };

  const formatUpdatedLabel = (updatedAt: number | null | undefined, fallback: string) => {
    return updatedAt ? `Updated: ${new Date(updatedAt * 1000).toLocaleTimeString()}` : fallback;
  };

  const ensureWalletProtectionReady = async (prompt: string) => {
    const passwordReady = await requestUnlock({
      mode: 'setup',
      reason: 'setup_required',
      prompt,
    });

    if (!passwordReady) {
      return false;
    }

    const backendState = await securityGetBackendState();
    const backendUnavailable = backendState && typeof backendState.backend_status === 'object' && 'unavailable' in backendState.backend_status;

    if (backendUnavailable) {
      throw 'secret_backend_unavailable';
    }

    return true;
  };

  const syncImportedWallet = async (wallet: WalletInfo) => {
    setWallets((current) => {
      const withoutWallet = current.filter((existing) => existing.id !== wallet.id);
      return [...withoutWallet, wallet];
    });

    try {
      const response = await invoke<BitcoinWalletBalanceResponse>('refresh_bitcoin_wallet_balance', { walletId: wallet.id });
      setWallets((current) => current.map((existing) => (
        existing.id === wallet.id ? response.wallet : existing
      )));
      setWalletBalanceStates((current) => new Map(current).set(wallet.id, response.balance_state));
    } catch (error) {
      console.error(`Error refreshing imported Bitcoin wallet ${wallet.id}:`, error);
      toast.warning('Wallet imported, but the first balance refresh did not complete. You can retry with Refresh.');
    }
  };

  const handleCreateMnemonic = async () => {
    setIsLoading(true);
    setWalletFlowRecovery(null);
    try {
      const ready = await ensureWalletProtectionReady('Set a local password before creating a Bitcoin wallet on this device.');
      if (!ready) {
        return;
      }

      const mnemonic = await invoke<string>('bitcoin_create_mnemonic');

      setPendingMnemonicBackup({
        mnemonic,
        walletLabel: walletLabel || 'Bitcoin Wallet',
      });
      setMnemonicInput('');
    } catch (error) {
      console.error('Error creating wallet:', error);
      setWalletFlowRecovery(describeWalletRecovery('create-wallet', error, { chainFamily: 'bitcoin' }));
    } finally {
      setIsLoading(false);
    }
  };

  const handleImportMnemonic = async () => {
    if (!mnemonicInput.trim()) return;

    setIsLoading(true);
    setWalletFlowRecovery(null);
    try {
      const ready = await ensureWalletProtectionReady('Set a local password before importing a Bitcoin wallet on this device.');
      if (!ready) {
        return;
      }

      const response = await invoke<CreateWalletResponse>('bitcoin_create_wallet_from_mnemonic', {
        mnemonicPhrase: mnemonicInput,
        walletLabel: walletLabel || undefined,
      });

      await syncImportedWallet(response.wallet);
      setMnemonicInput('');
      setWalletLabel('');
      setIsDialogOpen(false);
    } catch (error) {
      console.error('Error importing mnemonic:', error);
      setWalletFlowRecovery(describeWalletRecovery('import-wallet', error, { chainFamily: 'bitcoin' }));
    } finally {
      setIsLoading(false);
    }
  };

  const handleImportPrivateKey = async () => {
    if (!privateKeyInput.trim()) return;

    setIsLoading(true);
    setWalletFlowRecovery(null);
    try {
      const ready = await ensureWalletProtectionReady('Set a local password before importing a Bitcoin private key on this device.');
      if (!ready) {
        return;
      }

      const response = await invoke<CreateWalletResponse>('bitcoin_create_wallet_from_private_key', {
        privateKey: privateKeyInput,
        walletLabel: walletLabel || undefined,
      });

      await syncImportedWallet(response.wallet);
      setPrivateKeyInput('');
      setWalletLabel('');
      setIsDialogOpen(false);
    } catch (error) {
      console.error('Error importing private key:', error);
      setWalletFlowRecovery(describeWalletRecovery('import-wallet', error, { chainFamily: 'bitcoin' }));
    } finally {
      setIsLoading(false);
    }
  };

  const handleCopyAddress = (address: string) => {
    navigator.clipboard.writeText(address);
    setAddressCopied(address);
    setTimeout(() => setAddressCopied(null), 2000);
  };

  const handleCopyMnemonic = (mnemonic: string) => {
    navigator.clipboard.writeText(mnemonic);
    setMnemonicCopied(true);
    setTimeout(() => setMnemonicCopied(false), 2000);
  };

  const handleCopyExportedSecret = (secret: string) => {
    navigator.clipboard.writeText(secret);
    setExportCopied(true);
    setTimeout(() => setExportCopied(false), 2000);
  };

  const handleCloseMnemonicDialog = () => {
    setPendingMnemonicBackup(null);
    setMnemonicCopied(false);
  };

  const handlePersistCreatedWallet = async () => {
    if (!pendingMnemonicBackup) {
      return;
    }

    setIsPersistingMnemonic(true);
    setWalletFlowRecovery(null);
    try {
      const backendState = await securityGetBackendState();
      const backendUnavailable = backendState && typeof backendState.backend_status === 'object' && 'unavailable' in backendState.backend_status;
      if (backendUnavailable) {
        throw 'secret_backend_unavailable';
      }

      const response = await invoke<CreateWalletResponse>('bitcoin_create_wallet_from_mnemonic', {
        mnemonicPhrase: pendingMnemonicBackup.mnemonic,
        walletLabel: pendingMnemonicBackup.walletLabel || undefined,
      });

      await syncImportedWallet(response.wallet);
      setPendingMnemonicBackup(null);
      setMnemonicCopied(false);
      setWalletLabel('');
      setIsDialogOpen(false);
    } catch (error) {
      console.error('Error saving created wallet:', error);
      setWalletFlowRecovery(describeWalletRecovery('create-wallet', error, { chainFamily: 'bitcoin' }));
      toast.error('Wallet save stopped before local encryption completed.');
    } finally {
      setIsPersistingMnemonic(false);
    }
  };

  const handleExportPrivateKey = async (walletId: string) => {
    setIsLoading(true);
    try {
      const secret = await invoke<string>('bitcoin_export_private_key', { walletId });
      setExportedSecret(secret);
      setExportedSecretType('private-key');
      setShowExportDialog(true);
    } catch (error) {
      console.error('Error exporting private key:', error);
      toast.error(describeWalletRecovery('send-asset', error, { chainFamily: 'bitcoin' }).summary);
    } finally {
      setIsLoading(false);
    }
  };

  const handleExportMnemonic = async (walletId: string) => {
    setIsLoading(true);
    try {
      const secret = await invoke<string>('bitcoin_export_mnemonic', { walletId });
      setExportedSecret(secret);
      setExportedSecretType('mnemonic');
      setShowExportDialog(true);
    } catch (error) {
      console.error('Error exporting mnemonic:', error);
      toast.error(describeWalletRecovery('send-asset', error, { chainFamily: 'bitcoin' }).summary);
    } finally {
      setIsLoading(false);
    }
  };

  const handleDeleteWallet = async (walletId: string) => {
    setIsLoading(true);
    try {
      await invoke<boolean>('bitcoin_delete_wallet', { walletId });
      setWallets(wallets.filter(w => w.id !== walletId));
      setDeleteConfirm(null);
      setWalletBalanceStates(prev => {
        const next = new Map(prev);
        next.delete(walletId);
        return next;
      });
    } catch (error) {
      console.error('Error deleting wallet:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRefreshBalance = async (
    walletId: string,
    options?: {
      silent?: boolean;
      background?: boolean;
    }
  ) => {
    if (!options?.background) {
      setRefreshingBalance(walletId);
    }

    try {
      const response = await invoke<BitcoinWalletBalanceResponse>('refresh_bitcoin_wallet_balance', { walletId });
      setWallets(current => current.map(w => w.id === walletId ? response.wallet : w));
      setWalletBalanceStates(prev => new Map(prev).set(walletId, response.balance_state));
      if (!options?.silent && response.balance_state.freshness.status !== 'fresh') {
        toast.warning('BTC balance refresh is degraded. Showing the most honest state available.');
      }
    } catch (error) {
      console.error('Error refreshing balance:', error);
      if (!options?.silent) {
        alert(`Error refreshing balance: ${error}`);
      }
    } finally {
      if (!options?.background) {
        setRefreshingBalance(null);
      }
    }
  };

  const resetSendFlow = () => {
    setPendingSendReview(null);
    setSendToAddress('');
    setSendAmount('');
    setSendFeeRate(1);
    setEstimatedFees(null);
    setFeeRateType('half_hour');
    setIsSendAll(false);
    setSendFlowRecovery(null);
    setSelectedWalletForSend(null);
    setIsSendDialogOpen(false);
  };

  const buildCurrentBitcoinSendIntent = (): ReviewedBitcoinSendIntent => {
    if (!selectedWalletForSend || !sendToAddress || !sendAmount) {
      throw new Error('Complete the BTC send form before reviewing it');
    }

    const recipientAddress = sendToAddress.trim();
    if (!recipientAddress) {
      throw new Error('Enter a recipient address');
    }

    const amountSats = normalizeBtcAmountToSats(sendAmount);
    const amountBtcString = formatBtcFromSats(amountSats);
    const amountValue = Number(amountBtcString);
    const normalizedFeeRate = Math.max(1, Math.trunc(sendFeeRate || 1));
    const estimatedFeeSats = estimateBtcFeeSats(normalizedFeeRate);
    const totalChargeSats = BigInt(amountSats) + BigInt(estimatedFeeSats);

    const request = {
      wallet_id: selectedWalletForSend.id,
      to_address: recipientAddress,
      amount: amountValue,
      fee_rate: normalizedFeeRate,
      send_all: isSendAll,
    };

    return {
      actionLabel: 'Send BTC',
      realChainAction: 'Bitcoin transfer',
      walletLabel: selectedWalletForSend.label,
      fromAddress: selectedWalletForSend.address,
      recipientAddress,
      amountDisplay: `${amountBtcString} BTC`,
      amountSats,
      feeRateDisplay: `${normalizedFeeRate} sat/vB`,
      estimatedFeeDisplay: `${formatBtcFromSats(estimatedFeeSats.toString())} BTC`,
      totalChargeDisplay: `${formatBtcFromSats(totalChargeSats.toString())} BTC`,
      sendAll: isSendAll,
      riskPoint: isSendAll
        ? 'Send-all spends the wallet balance using the reviewed fee rate, leaving no intentional remainder in this wallet.'
        : 'Bitcoin sends are irreversible once broadcast, so recipient and fee rate must match the reviewed transfer.',
      request,
      payloadFingerprint: buildBitcoinPayloadFingerprint(request),
    };
  };

  const handlePrepareSendReview = () => {
    try {
      setSendFlowRecovery(null);
      const reviewedIntent = buildCurrentBitcoinSendIntent();
      setPendingSendReview(reviewedIntent);
    } catch (error) {
      const message = typeof error === 'string' ? error : error instanceof Error ? error.message : 'Unable to review BTC send';
      toast.error(message);
    }
  };

  const handleReturnToSendEdit = () => {
    setPendingSendReview(null);
  };

  const submitReviewedSend = async () => {
    if (!pendingSendReview) return;

    if (!isTauriRuntimeAvailable()) {
      toast.error(TAURI_UNAVAILABLE_MESSAGE);
      return;
    }

    let currentReviewedIntent: ReviewedBitcoinSendIntent;
    try {
      currentReviewedIntent = buildCurrentBitcoinSendIntent();
    } catch (error) {
      setPendingSendReview(null);
      const message = typeof error === 'string' ? error : error instanceof Error ? error.message : 'Unable to validate the current BTC send payload';
      toast.error(message);
      return;
    }

    if (currentReviewedIntent.payloadFingerprint !== pendingSendReview.payloadFingerprint) {
      setPendingSendReview(null);
      toast.error('BTC send payload changed. Review the send again.');
      return;
    }

    setIsSending(true);
    setSendFlowRecovery(null);
    try {
      const response = await invoke<{ tx_hash: string; message: string }>('send_bitcoin', {
        request: pendingSendReview.request
      });

      toast.success(
        <div className="flex flex-col gap-1">
          <div className="font-medium">Transaction Sent Successfully</div>
          <div className="text-[10px] font-mono opacity-70 break-all">{response.tx_hash}</div>
          <button
            onClick={() => openExternalLink(getBitcoinExplorerUrl(response.tx_hash))}
            className="flex items-center gap-1 text-[10px] text-white underline mt-1 hover:no-underline"
          >
            <ExternalLink className="w-3 h-3" />
            View on Explorer
          </button>
        </div>,
        { duration: 10000 }
      );

      setPendingSendReview(null);

      // Refresh balance after successful send
      handleRefreshBalance(pendingSendReview.request.wallet_id);
      resetSendFlow();
    } catch (error) {
      console.error('Error sending BTC:', error);
      if (isTauriUnavailableError(error)) {
        toast.error(TAURI_UNAVAILABLE_MESSAGE);
        return;
      }

      const securityError = parseSecurityError(error);
      if (securityError === 'locked' || securityError === 'expired' || securityError === 'reauth_required') {
        setSendFlowRecovery(describeWalletRecovery('send-asset', error, { chainFamily: 'bitcoin' }));
        void requestUnlock({
          prompt: 'Re-enter your local password to send BTC.',
          reason: securityError === 'reauth_required' ? 'reauth_required' : securityError,
          mode: 'reauth',
          operation: 'send',
          onUnlockSuccess: submitReviewedSend,
        });
      } else {
        setSendFlowRecovery(describeWalletRecovery('send-asset', error, { chainFamily: 'bitcoin' }));
      }
    } finally {
      setIsSending(false);
    }
  };

  const handleConfirmReviewedSend = async () => {
    if (!pendingSendReview) {
      return;
    }

    await submitReviewedSend();
  };

  const openSendDialog = (wallet: WalletInfo) => {
    setSelectedWalletForSend(wallet);
    setPendingSendReview(null);
    setSendToAddress('');
    setSendAmount('');
    setEstimatedFees(null);
    setFeeRateType('half_hour');
    setSendFeeRate(1);
    setIsSendAll(false);
    setIsSendDialogOpen(true);
  };

  const totalBalance = wallets.reduce((sum, wallet) => sum + wallet.balance, 0);

  return (
    <Card className="p-6 select-none glass-card">
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="p-3 bg-gradient-to-br from-orange-400 to-orange-600 rounded-2xl shadow-lg shadow-orange-500/20">
              <img src="/images/assets/bitcoin.png" alt="Bitcoin" className="w-8 h-8" />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-foreground">Bitcoin</h3>
              <p className="text-xs text-muted-foreground font-medium">{wallets.length} wallet{wallets.length !== 1 ? 's' : ''}</p>
            </div>
          </div>
          <Dialog open={isDialogOpen} onOpenChange={(open) => {
            setIsDialogOpen(open);
            if (!open) {
              setWalletFlowRecovery(null);
            }
          }}>
            <DialogTrigger asChild>
              <Button className="gap-2">
                <Plus className="w-4 h-4" />
                Add / Import Wallet
              </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-[500px]">
              <DialogHeader>
                <DialogTitle>Bitcoin Wallet Management</DialogTitle>
              </DialogHeader>
              {walletFlowRecovery && (
                <RecoveryPanel guidance={walletFlowRecovery} />
              )}
              <Tabs defaultValue="create" className="w-full">
                <TabsList className="grid w-full grid-cols-3">
                  <TabsTrigger value="create">Create New</TabsTrigger>
                  <TabsTrigger value="mnemonic">Import Mnemonic</TabsTrigger>
                  <TabsTrigger value="private-key">Import Private Key</TabsTrigger>
                </TabsList>

                {/* Create New Wallet */}
                <TabsContent value="create" className="space-y-4 mt-4">
                  <p className="text-sm text-muted-foreground">
                    Create a new wallet with a secure mnemonic phrase.
                  </p>
                  <div className="space-y-2">
                    <Label htmlFor="wallet-label-create">Wallet Label (Optional)</Label>
                    <Input
                      id="wallet-label-create"
                      placeholder="e.g., My Main Wallet"
                      value={walletLabel}
                      onChange={(e) => setWalletLabel(e.target.value)}
                    />
                  </div>
                  <Button
                    onClick={handleCreateMnemonic}
                    className="w-full"
                    disabled={isLoading}
                  >
                    {isLoading ? 'Creating...' : 'Create New Wallet'}
                  </Button>
                </TabsContent>

                {/* Import Mnemonic */}
                <TabsContent value="mnemonic" className="space-y-4 mt-4">
                  <div className="space-y-2">
                    <Label htmlFor="wallet-label-mnemonic">Wallet Label (Optional)</Label>
                    <Input
                      id="wallet-label-mnemonic"
                      placeholder="e.g., Imported Wallet"
                      value={walletLabel}
                      onChange={(e) => setWalletLabel(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="mnemonic">Mnemonic Phrase</Label>
                    <Textarea
                      id="mnemonic"
                      placeholder="Enter your 12 or 24 word mnemonic phrase..."
                      value={mnemonicInput}
                      onChange={(e) => setMnemonicInput(e.target.value)}
                      rows={4}
                      className="font-mono text-sm"
                    />
                    <p className="text-xs text-muted-foreground">
                      Words should be separated by spaces
                    </p>
                  </div>
                  <Button
                    onClick={handleImportMnemonic}
                    className="w-full"
                    disabled={!mnemonicInput.trim() || isLoading}
                  >
                    {isLoading ? 'Importing...' : 'Import Mnemonic'}
                  </Button>
                </TabsContent>

                {/* Import Private Key */}
                <TabsContent value="private-key" className="space-y-4 mt-4">
                  <div className="space-y-2">
                    <Label htmlFor="wallet-label-key">Wallet Label (Optional)</Label>
                    <Input
                      id="wallet-label-key"
                      placeholder="e.g., Hardware Wallet"
                      value={walletLabel}
                      onChange={(e) => setWalletLabel(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="private-key">Private Key</Label>
                    <Textarea
                      id="private-key"
                      placeholder="Enter your private key (WIF or 64-char hex format)..."
                      value={privateKeyInput}
                      onChange={(e) => setPrivateKeyInput(e.target.value)}
                      rows={3}
                      className="font-mono text-sm"
                    />
                    <p className="text-xs text-muted-foreground">
                      Support WIF format (e.g., K...) or 64-character hex string
                    </p>
                  </div>
                  <Button
                    onClick={handleImportPrivateKey}
                    className="w-full"
                    disabled={!privateKeyInput.trim() || isLoading}
                  >
                    {isLoading ? 'Importing...' : 'Import Private Key'}
                  </Button>
                </TabsContent>
              </Tabs>
            </DialogContent>
          </Dialog>
        </div>

        <MnemonicBackupDialog
          open={pendingMnemonicBackup !== null}
          chainLabel="Bitcoin"
          walletLabel={pendingMnemonicBackup?.walletLabel ?? ''}
          mnemonic={pendingMnemonicBackup?.mnemonic ?? null}
          copied={mnemonicCopied}
          isSaving={isPersistingMnemonic}
          onCopy={handleCopyMnemonic}
          onCancel={handleCloseMnemonicDialog}
          onConfirm={handlePersistCreatedWallet}
        />

        {/* Export Secret Dialog */}
        <Dialog open={showExportDialog} onOpenChange={setShowExportDialog}>
          <DialogContent className="sm:max-w-[600px]">
            <DialogHeader>
              <DialogTitle>
                {exportedSecretType === 'mnemonic' ? 'Export Mnemonic Phrase' : 'Export Private Key'}
              </DialogTitle>
            </DialogHeader>

            {exportedSecret && (
              <div className="space-y-4">
                {/* Warning Alert */}
                <div className="bg-red-50 border border-red-200 rounded-lg p-4 space-y-2">
                  <div className="flex items-start gap-3">
                    <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
                    <div className="space-y-1">
                      <p className="font-semibold text-red-900 text-sm">Keep this information secure!</p>
                      <p className="text-xs text-red-800">
                        Anyone with access to this {exportedSecretType === 'mnemonic' ? 'mnemonic phrase' : 'private key'} can control your funds. Do not share or screenshot it.
                      </p>
                    </div>
                  </div>
                </div>

                {/* Secret Display */}
                <div className="space-y-2">
                  <Label>
                    {exportedSecretType === 'mnemonic' ? 'Mnemonic Phrase' : 'Private Key'}
                  </Label>
                  <div className="bg-muted rounded-lg p-4">
                    {exportedSecretType === 'mnemonic' ? (
                      <div className="grid grid-cols-3 gap-2 mb-3">
                        {exportedSecret.split(' ').map((word, index) => (
                          <div key={index} className="flex items-center gap-2">
                            <span className="text-muted-foreground text-xs font-mono w-6">{index + 1}.</span>
                            <span className="text-yellow-400 font-mono text-sm break-all">{word}</span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p className="text-yellow-400 font-mono text-sm break-all">{exportedSecret}</p>
                    )}
                    <button
                      onClick={() => handleCopyExportedSecret(exportedSecret)}
                      className="w-full mt-3 px-3 py-2 bg-muted/80 hover:bg-muted/70 text-gray-200 rounded text-sm transition-colors flex items-center justify-center gap-2"
                    >
                      {exportCopied ? (
                        <>
                          <CheckCircle2 className="w-4 h-4" />
                          Copied!
                        </>
                      ) : (
                        <>
                          <Copy className="w-4 h-4" />
                          Copy to Clipboard
                        </>
                      )}
                    </button>
                  </div>
                </div>

                {/* Close Button */}
                <Button
                  className="w-full"
                  onClick={() => setShowExportDialog(false)}
                >
                  Close
                </Button>
              </div>
            )}
          </DialogContent>
        </Dialog>

        {/* Send Bitcoin Dialog */}
        <Dialog open={isSendDialogOpen} onOpenChange={(open) => {
          setIsSendDialogOpen(open);
          if (!open) {
            resetSendFlow();
          }
        }}>
          <DialogContent className={pendingSendReview ? "sm:max-w-[520px] border-none shadow-2xl bg-[#161821] text-white" : "sm:max-w-[450px]"}>
            <DialogHeader>
              <DialogTitle>{pendingSendReview?.actionLabel ?? 'Send Bitcoin'}</DialogTitle>
            </DialogHeader>
            {!pendingSendReview ? (
            <div className="space-y-4 py-4">
              {sendFlowRecovery && (
                <RecoveryPanel guidance={sendFlowRecovery} className="text-left" />
              )}
              <div className="space-y-2">
                <Label>From Wallet</Label>
                <div className="p-3 bg-muted/50 rounded-lg text-sm border border-border/50">
                  <p className="font-medium text-foreground">{selectedWalletForSend?.label}</p>
                  <p className="font-mono text-xs text-muted-foreground break-all">{selectedWalletForSend?.address}</p>
                  <div className="flex items-center justify-between mt-2 pt-2 border-t border-border/50">
                    <p className="text-xs text-muted-foreground uppercase tracking-wider font-semibold">Balance</p>
                    <p className="text-xs font-mono font-bold text-orange-600">{truncateBtc(selectedWalletForSend?.balance)} BTC</p>
                  </div>
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="to-address">Recipient Address</Label>
                <Input
                  id="to-address"
                  placeholder="Enter Bitcoin address (e.g., bc1...)"
                  value={sendToAddress}
                  onChange={(e) => setSendToAddress(e.target.value)}
                  className="font-mono text-sm"
                />
              </div>
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="amount">Amount (BTC)</Label>
                  {btcPrice.price_usd !== null && sendAmount && !isNaN(parseFloat(sendAmount)) && (
                    <span className="text-[10px] text-muted-foreground font-mono">
                      ≈ ${(parseFloat(sendAmount) * btcPrice.price_usd).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                    </span>
                  )}
                </div>
                <div className="relative">
                  <Input
                    id="amount"
                    type="number"
                    step="0.00000001"
                    placeholder="0.00000000"
                    value={sendAmount}
                    onChange={(e) => {
                      setSendAmount(e.target.value);
                      setIsSendAll(false);
                    }}
                    className="font-mono text-sm"
                  />
                  <Button
                    variant="ghost"
                    size="sm"
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] h-7 px-2 hover:bg-orange-50 hover:text-orange-600"
                    onClick={() => {
                      setSendAmount((selectedWalletForSend?.balance || 0).toString());
                      setIsSendAll(true);
                    }}
                  >
                    MAX
                  </Button>
                </div>
              </div>
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <Label htmlFor="fee-rate" className="text-xs">Fee Rate (sat/vB)</Label>
                    <HelpCircle className="w-3 h-3 text-muted-foreground" />
                  </div>
                  <div className="flex gap-1">
                    <button
                      onClick={() => setFeeRateType('hour')}
                      className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${feeRateType === 'hour' ? 'bg-orange-500/10 border-orange-500/50 text-orange-500' : 'bg-muted/50 border-border/50 text-muted-foreground'}`}
                    >
                      Slow
                    </button>
                    <button
                      onClick={() => setFeeRateType('half_hour')}
                      className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${feeRateType === 'half_hour' ? 'bg-orange-500/10 border-orange-500/50 text-orange-500' : 'bg-muted/50 border-border/50 text-muted-foreground'}`}
                    >
                      Avg
                    </button>
                    <button
                      onClick={() => setFeeRateType('fast')}
                      className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${feeRateType === 'fast' ? 'bg-orange-500/10 border-orange-500/50 text-orange-500' : 'bg-muted/50 border-border/50 text-muted-foreground'}`}
                    >
                      Fast
                    </button>
                    <button
                      onClick={() => setFeeRateType('custom')}
                      className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${feeRateType === 'custom' ? 'bg-orange-500/10 border-orange-500/50 text-orange-500' : 'bg-muted/50 border-border/50 text-muted-foreground'}`}
                    >
                      Custom
                    </button>
                  </div>
                </div>

                <div className="relative">
                  <Input
                    id="fee-rate"
                    type="number"
                    min="1"
                    value={sendFeeRate}
                    onChange={(e) => {
                      setSendFeeRate(parseInt(e.target.value) || 1);
                      setFeeRateType('custom');
                    }}
                    className="font-mono text-sm pr-16"
                  />
                  <div className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-muted-foreground font-mono">
                    sat/vB
                  </div>
                </div>

                {isEstimatingFees && (
                  <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
                    <RefreshCw className="w-2.5 h-2.5 animate-spin" />
                    Updating network fees...
                  </div>
                )}

                <div className="p-3 bg-orange-500/5 rounded-xl border border-orange-500/10 space-y-2">
                  <div className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Network Fee</span>
                    <div className="text-right">
                      <p className="text-xs font-bold text-white">
                        {/* Rough estimation for common transactions (input + 2 outputs) */}
                        {truncateBtc((148 * sendFeeRate) / 100_000_000)} BTC
                      </p>
                      {btcPrice.price_usd !== null && (
                        <p className="text-[10px] text-muted-foreground">
                          ≈ ${((148 * sendFeeRate) / 100_000_000 * btcPrice.price_usd).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </p>
                      )}
                    </div>
                  </div>
                  <div className="h-px bg-white/5" />
                  <div className="flex justify-between items-center">
                    <span className="text-sm font-bold text-white">Total Charge</span>
                    <div className="text-right">
                      <p className="text-sm font-bold text-orange-400">
                        {truncateBtc(parseFloat(sendAmount || '0') + (148 * sendFeeRate) / 100_000_000)} BTC
                      </p>
                    </div>
                  </div>
                </div>
              </div>
              <div className="flex gap-3">
                <Button variant="outline" className="flex-1" onClick={() => setIsSendDialogOpen(false)}>
                  Cancel
                </Button>
                <Button
                  className="flex-1 bg-orange-600 hover:bg-orange-700 shadow-lg shadow-orange-500/20"
                  onClick={handlePrepareSendReview}
                  disabled={isSending || !sendToAddress || !sendAmount || parseFloat(sendAmount) <= 0}
                >
                  {isSending ? (
                    <>
                      <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                      Processing...
                    </>
                  ) : (
                    <>
                      <Send className="w-4 h-4 mr-2" />
                      Review Send
                    </>
                  )}
                </Button>
              </div>
            </div>
            ) : (
            <div className="space-y-5 py-2">
              {sendFlowRecovery && (
                <RecoveryPanel guidance={sendFlowRecovery} className="text-left" />
              )}
              <div className="rounded-xl border border-orange-500/20 bg-orange-500/10 p-4 space-y-2">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-xs uppercase tracking-[0.2em] text-orange-200/80">Real Chain Action</span>
                  <span className="text-sm font-semibold text-orange-50">{pendingSendReview.realChainAction}</span>
                </div>
                <p className="text-sm text-slate-200">{pendingSendReview.riskPoint}</p>
              </div>

              <div className="grid gap-3 sm:grid-cols-2">
                <div className="rounded-xl border border-white/10 bg-white/5 p-3">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">From Wallet</p>
                  <p className="mt-1 text-sm font-semibold text-white">{pendingSendReview.walletLabel}</p>
                  <p className="mt-1 break-all font-mono text-xs text-slate-300">{pendingSendReview.fromAddress}</p>
                </div>
                <div className="rounded-xl border border-white/10 bg-white/5 p-3">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">Recipient</p>
                  <p className="mt-1 break-all font-mono text-sm text-white">{pendingSendReview.recipientAddress}</p>
                </div>
                <div className="rounded-xl border border-white/10 bg-white/5 p-3">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">Amount</p>
                  <p className="mt-1 text-sm font-semibold text-white">{pendingSendReview.amountDisplay}</p>
                  <p className="mt-1 font-mono text-xs text-slate-300">{pendingSendReview.amountSats} sats</p>
                </div>
                <div className="rounded-xl border border-white/10 bg-white/5 p-3">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">Fee Rate</p>
                  <p className="mt-1 font-mono text-sm text-white">{pendingSendReview.feeRateDisplay}</p>
                </div>
                <div className="rounded-xl border border-white/10 bg-white/5 p-3">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">Estimated Network Fee</p>
                  <p className="mt-1 font-mono text-sm text-white">{pendingSendReview.estimatedFeeDisplay}</p>
                </div>
                <div className="rounded-xl border border-white/10 bg-white/5 p-3">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">Total Charge</p>
                  <p className="mt-1 font-mono text-sm text-white">{pendingSendReview.totalChargeDisplay}</p>
                </div>
                <div className="rounded-xl border border-white/10 bg-white/5 p-3 sm:col-span-2">
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-400">Send-All Mode</p>
                  <p className="mt-1 text-sm font-semibold text-white">{pendingSendReview.sendAll ? 'Enabled' : 'Disabled'}</p>
                  <p className="mt-1 text-xs text-slate-300">
                    {pendingSendReview.sendAll
                      ? 'The reviewed payload is marked as send-all and will spend using the reviewed fee rate.'
                      : 'The reviewed payload sends only the typed BTC amount.'}
                  </p>
                </div>
              </div>

              <div className="flex gap-3 pt-2">
                <Button
                  variant="outline"
                  className="flex-1 border-slate-600 text-slate-200 hover:bg-slate-800"
                  onClick={handleReturnToSendEdit}
                  disabled={isSending}
                >
                  Back
                </Button>
                <Button
                  className="flex-1 bg-orange-600 hover:bg-orange-700 text-white"
                  onClick={handleConfirmReviewedSend}
                  disabled={isSending}
                >
                  {isSending ? 'Sending...' : 'Confirm Send'}
                </Button>
              </div>
            </div>
            )}
          </DialogContent>
        </Dialog>

        {/* Total Balance */}
        {wallets.length > 0 && (
          <div className="bg-muted/30 rounded-lg p-4 border border-border/50">
            <p className="text-xs text-muted-foreground font-medium mb-2 uppercase tracking-wide">Total Balance</p>
            <div className="flex items-baseline gap-3">
              <p className="text-3xl font-light tracking-tight text-foreground font-mono">
                {truncateBtc(totalBalance)} BTC
              </p>
              {btcPrice.price_usd !== null && (
                <p className="text-sm text-muted-foreground font-mono">
                  ${(totalBalance * btcPrice.price_usd).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </p>
              )}
            </div>
            <div className="mt-3 flex items-center gap-2 text-[10px] uppercase tracking-wide">
              <Badge variant="outline" className={getPriceBadgeClass(btcPrice.status)}>
                {getPriceStatusLabel(btcPrice.status)}
              </Badge>
              <span className="text-muted-foreground normal-case tracking-normal">
                {formatUpdatedLabel(btcPrice.price_updated_at, btcPrice.status === 'unavailable' ? 'Price unavailable' : 'Price status pending')}
              </span>
            </div>
            <p className="mt-2 text-xs text-muted-foreground max-w-xl">
              {getPriceStatusDescription(btcPrice)}
            </p>
          </div>
        )}

        {/* Wallets List */}
        <div className="space-y-3">
          {wallets.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-muted-foreground mb-4 text-sm">No wallets yet</p>
              <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
                <DialogTrigger asChild>
                  <Button variant="outline" className="gap-2">
                    <Plus className="w-4 h-4" />
                    Add Your First Wallet
                  </Button>
                </DialogTrigger>
              </Dialog>
            </div>
          ) : (
            wallets.map((wallet) => {
              const balanceState = walletBalanceStates.get(wallet.id);
              const balanceFreshness = balanceState?.freshness;

              return (
              <div
                key={wallet.id}
                className="border border-border rounded-lg p-4 hover:bg-muted/30 transition-colors space-y-3"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-foreground text-sm">
                        {wallet.label}
                      </p>
                      <Badge variant="secondary" className="text-xs">
                        {wallet.wallet_type === 'mnemonic' ? 'Mnemonic' : 'Private Key'}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2 mt-2">
                      <p className="font-mono text-xs text-muted-foreground truncate">
                        {shortAddress(wallet.address)}
                      </p>
                      <button
                        onClick={() => handleCopyAddress(wallet.address)}
                        className="text-muted-foreground hover:text-foreground transition-colors flex-shrink-0"
                        title="Copy address"
                      >
                        {addressCopied === wallet.address ? (
                          <CheckCircle2 className="w-4 h-4 text-green-600" />
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                  </div>
                  <div className="text-right ml-4 flex flex-col items-end gap-2">
                    <div>
                      <p className="font-semibold text-base text-foreground font-mono">
                        {truncateBtc(wallet.balance)} BTC
                      </p>
                      {btcPrice.price_usd !== null && (
                        <p className="text-xs text-muted-foreground mt-1 font-mono">
                          ${(wallet.balance * btcPrice.price_usd).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </p>
                      )}
                      <div className="mt-2 flex items-center justify-end gap-2 text-[10px] uppercase tracking-wide">
                        {balanceFreshness && (
                          <Badge variant="outline" className={getFreshnessBadgeClass(balanceFreshness.status)}>
                            {formatFreshnessLabel(balanceFreshness.status)}
                          </Badge>
                        )}
                        <span className="text-muted-foreground normal-case tracking-normal">
                          {formatUpdatedLabel(balanceFreshness?.updated_at, balanceFreshness?.status === 'cached' ? 'Cached state' : 'Balance status pending')}
                        </span>
                      </div>
                    </div>
                    {/* Action Buttons */}
                    <div className="flex gap-1 flex-wrap justify-end">
                      {/* Send Button */}
                      <button
                        onClick={() => openSendDialog(wallet)}
                        className="px-2 py-1 text-xs bg-orange-50 text-orange-700 hover:bg-orange-100 rounded transition-colors flex items-center gap-1"
                        title="Send Bitcoin"
                        disabled={isLoading}
                      >
                        <Send className="w-3 h-3" />
                        <span>Send</span>
                      </button>

                      {/* Refresh Balance Button */}
                      <button
                        onClick={() => handleRefreshBalance(wallet.id)}
                        className="px-2 py-1 text-xs bg-purple-50 text-purple-700 hover:bg-purple-100 rounded transition-colors flex items-center gap-1"
                        title="Refresh balance from blockchain"
                        disabled={refreshingBalance === wallet.id}
                      >
                        <RefreshCw className={`w-3 h-3 ${refreshingBalance === wallet.id ? 'animate-spin' : ''}`} />
                        <span>{refreshingBalance === wallet.id ? 'Refreshing...' : 'Refresh'}</span>
                      </button>

                      {/* Export Private Key Button */}
                      <UnlockGate
                        mode="reauth"
                        operation="export_private_key"
                        prompt="Re-enter your local password to reveal the Bitcoin private key."
                        onUnlockSuccess={() => handleExportPrivateKey(wallet.id)}
                      >
                        <button
                          onClick={() => handleExportPrivateKey(wallet.id)}
                          className="px-2 py-1 text-xs bg-slate-100 text-slate-700 hover:bg-slate-200 rounded transition-colors flex items-center gap-1"
                          title="Export private key"
                          disabled={isLoading}
                        >
                          <Download className="w-3 h-3" />
                          <span>Export Private Key</span>
                        </button>
                      </UnlockGate>

                      {/* Export Mnemonic Button (only for mnemonic wallets) */}
                      {wallet.wallet_type === 'mnemonic' && (
                        <UnlockGate
                          mode="reauth"
                          operation="export_mnemonic"
                          prompt="Re-enter your local password to reveal the Bitcoin recovery phrase."
                          onUnlockSuccess={() => handleExportMnemonic(wallet.id)}
                        >
                          <button
                            onClick={() => handleExportMnemonic(wallet.id)}
                            className="px-2 py-1 text-xs bg-slate-100 text-slate-700 hover:bg-slate-200 rounded transition-colors flex items-center gap-1"
                            title="Export mnemonic"
                            disabled={isLoading}
                          >
                            <Download className="w-3 h-3" />
                            <span>Export Mnemonic</span>
                          </button>
                        </UnlockGate>
                      )}

                      {/* Delete Button */}
                      {deleteConfirm === wallet.id ? (
                        <>
                          <button
                            onClick={() => setDeleteConfirm(null)}
                            className="px-2 py-1 text-xs text-muted-foreground hover:bg-muted rounded"
                          >
                            Cancel
                          </button>
                          <button
                            onClick={() => handleDeleteWallet(wallet.id)}
                            className="px-2 py-1 text-xs bg-red-100 text-red-700 hover:bg-red-200 rounded transition-colors"
                            disabled={isLoading}
                          >
                            {isLoading ? 'Deleting...' : 'Confirm'}
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={() => setDeleteConfirm(wallet.id)}
                          className="px-2 py-1 text-xs text-red-600 hover:bg-red-50 rounded transition-colors flex items-center gap-1"
                          title="Delete wallet"
                          disabled={isLoading}
                        >
                          <Trash2 className="w-3 h-3" />
                          <span>Delete</span>
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            )})
          )}
        </div>
      </div>
    </Card>
  );
};

export default BitcoinAssets;
