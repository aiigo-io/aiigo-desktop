import React, { useState, useEffect } from 'react';
import { Card, Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger, Tabs, TabsContent, TabsList, TabsTrigger, Label, Textarea, Input, Badge, Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui';
import { Copy, Plus, AlertCircle, CheckCircle2, Trash2, Download, Send, ChevronRight, HelpCircle, ExternalLink } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { shortAddress, getEvmExplorerUrl, openExternalLink } from '@/lib/utils';
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

interface EvmAsset {
  symbol: string;
  name: string;
  decimals: number;
  contract_address: string | null;
}

interface EvmAssetBalance {
  chain: string;
  asset: EvmAsset;
  balance: string;
  balance_float: number;
  usd_price: number;
  usd_value: number;
}

interface EvmChainAssets {
  chain: string;
  chain_id: number;
  total_balance_usd: number;
  assets: EvmAssetBalance[];
}

interface EvmWalletInfo {
  id: string;
  label: string;
  wallet_type: 'mnemonic' | 'private-key';
  address: string;
  chains: EvmChainAssets[];
  total_balance_usd: number;
  created_at: string;
  updated_at: string;
}

interface CreateWalletResponse {
  mnemonic: string;
  wallet: WalletInfo;
}

const REFRESH_CACHE_TTL_MS = 60_000; // Align with backend price cache duration
type RefreshScope = 'single' | 'all';

const EvmAssets: React.FC = () => {
  const [wallets, setWallets] = useState<WalletInfo[]>([]);
  const [walletsWithBalances, setWalletsWithBalances] = useState<Map<string, EvmWalletInfo>>(new Map());
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [showMnemonicDialog, setShowMnemonicDialog] = useState(false);
  const [generatedMnemonic, setGeneratedMnemonic] = useState<CreateWalletResponse | null>(null);
  const [mnemonicInput, setMnemonicInput] = useState('');
  const [privateKeyInput, setPrivateKeyInput] = useState('');
  const [walletLabel, setWalletLabel] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [mnemonicCopied, setMnemonicCopied] = useState(false);
  const [exportedSecret, setExportedSecret] = useState<string | null>(null);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [exportedSecretType, setExportedSecretType] = useState<'mnemonic' | 'private-key'>('private-key');
  const [exportCopied, setExportCopied] = useState(false);
  const [addressCopied, setAddressCopied] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshingWalletId, setRefreshingWalletId] = useState<string | null>(null);
  const [lastRefreshTime, setLastRefreshTime] = useState<Map<string, Date>>(new Map());

  // Send EVM State
  const [isSendDialogOpen, setIsSendDialogOpen] = useState(false);
  const [selectedWalletForSend, setSelectedWalletForSend] = useState<WalletInfo | null>(null);
  const [selectedAssetForSend, setSelectedAssetForSend] = useState<{ chain: string, chainId: number, asset: EvmAssetBalance } | null>(null);
  const [sendToAddress, setSendToAddress] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [isSending, setIsSending] = useState(false);

  // Gas Estimation State
  const [isEstimatingGas, setIsEstimatingGas] = useState(false);
  const [estimatedGasLimit, setEstimatedGasLimit] = useState<number | null>(null);
  const [estimatedGasPrice, setEstimatedGasPrice] = useState<string | null>(null); // wei as string
  const [gasEstimationError, setGasEstimationError] = useState<string | null>(null);

  // Load wallets on mount
  useEffect(() => {
    loadWallets();
  }, []);

  // Effect for gas estimation
  useEffect(() => {
    const estimateGas = async () => {
      if (!selectedWalletForSend || !selectedAssetForSend || !sendToAddress || !sendAmount || parseFloat(sendAmount) <= 0) {
        setEstimatedGasLimit(null);
        setEstimatedGasPrice(null);
        setGasEstimationError(null);
        return;
      }

      // Basic address validation
      if (!sendToAddress.startsWith('0x') || sendToAddress.length !== 42) {
        setGasEstimationError('Invalid address');
        return;
      }

      setIsEstimatingGas(true);
      setGasEstimationError(null);

      try {
        const decimals = selectedAssetForSend.asset.asset.decimals;
        const amountFloat = parseFloat(sendAmount);
        const amountInBaseUnits = BigInt(Math.floor(amountFloat * Math.pow(10, decimals))).toString();

        const estimation = await invoke<{ gas_limit: number; gas_price: string }>('evm_estimate_gas', {
          request: {
            wallet_id: selectedWalletForSend.id,
            to_address: sendToAddress,
            amount: amountInBaseUnits,
            chain: selectedAssetForSend.chain,
            chain_id: selectedAssetForSend.chainId,
            asset_symbol: selectedAssetForSend.asset.asset.symbol,
            contract_address: selectedAssetForSend.asset.asset.contract_address,
            gas_limit: null,
            gas_price: null,
          }
        });

        setEstimatedGasLimit(estimation.gas_limit);
        setEstimatedGasPrice(estimation.gas_price);
      } catch (error) {
        console.error('Error estimating gas:', error);
        setGasEstimationError(typeof error === 'string' ? error : 'Estimation failed');
        setEstimatedGasLimit(null);
        setEstimatedGasPrice(null);
      } finally {
        setIsEstimatingGas(false);
      }
    };

    const timer = setTimeout(estimateGas, 500); // Debounce estimation
    return () => clearTimeout(timer);
  }, [selectedWalletForSend, selectedAssetForSend, sendToAddress, sendAmount]);

  const isCacheHit = (walletId: string) => {
    const lastRefresh = lastRefreshTime.get(walletId);
    if (!lastRefresh) return false;
    return Date.now() - lastRefresh.getTime() < REFRESH_CACHE_TTL_MS;
  };

  const logRefreshMetrics = ({
    walletId,
    scope,
    cacheHit,
    durationMs,
    error,
  }: {
    walletId: string;
    scope: RefreshScope;
    cacheHit: boolean;
    durationMs: number;
    error?: unknown;
  }) => {
    const payload = {
      walletId,
      scope,
      cacheHit,
      durationMs: Number(durationMs.toFixed(2)),
      timestamp: new Date().toISOString(),
    };

    if (error) {
      console.error('[EVM Refresh]', payload, error);
    } else {
      console.info('[EVM Refresh]', payload);
    }
  };

  const fetchWalletWithMetrics = async (
    walletId: string,
    scope: RefreshScope
  ): Promise<EvmWalletInfo> => {
    const cacheHit = isCacheHit(walletId);
    const start = performance.now();
    try {
      const walletWithBalances = await invoke<EvmWalletInfo>('evm_get_wallet_with_balances', { walletId });
      logRefreshMetrics({
        walletId,
        scope,
        cacheHit,
        durationMs: performance.now() - start,
      });
      return walletWithBalances;
    } catch (error) {
      logRefreshMetrics({
        walletId,
        scope,
        cacheHit,
        durationMs: performance.now() - start,
        error,
      });
      throw error;
    }
  };

  const loadWallets = async () => {
    try {
      const result = await invoke<WalletInfo[]>('evm_get_wallets');
      setWallets(result);

      // Load balance data for each wallet
      const balances = new Map<string, EvmWalletInfo>();
      for (const wallet of result) {
        try {
          const walletWithBalances = await invoke<EvmWalletInfo>('evm_get_wallet_with_balances', { walletId: wallet.id });
          balances.set(wallet.id, walletWithBalances);
          setLastRefreshTime(prev => new Map(prev).set(wallet.id, new Date()));
        } catch (error) {
          console.error(`Error loading balances for wallet ${wallet.id}:`, error);
        }
      }
      setWalletsWithBalances(balances);
    } catch (error) {
      console.error('Error loading wallets:', error);
    }
  };

  const handleRefreshBalance = async (walletId?: string) => {
    setIsRefreshing(true);
    try {
      if (walletId) {
        // Refresh specific wallet
        setRefreshingWalletId(walletId);
        const walletWithBalances = await fetchWalletWithMetrics(walletId, 'single');
        setWalletsWithBalances(prev => new Map(prev).set(walletId, walletWithBalances));
        setLastRefreshTime(prev => new Map(prev).set(walletId, new Date()));
      } else {
        // Refresh all wallets
        const balances = new Map<string, EvmWalletInfo>();
        const refreshTimes = new Map<string, Date>();
        for (const wallet of wallets) {
          try {
            const walletWithBalances = await fetchWalletWithMetrics(wallet.id, 'all');
            balances.set(wallet.id, walletWithBalances);
            refreshTimes.set(wallet.id, new Date());
          } catch (error) {
            console.error(`Error refreshing balances for wallet ${wallet.id}:`, error);
          }
        }
        setWalletsWithBalances(balances);
        setLastRefreshTime(refreshTimes);
      }
    } catch (error) {
      console.error('Error refreshing balance:', error);
      alert(`Error refreshing balance: ${error}`);
    } finally {
      setIsRefreshing(false);
      setRefreshingWalletId(null);
    }
  };

  const handleCreateMnemonic = async () => {
    setIsLoading(true);
    try {
      const mnemonic = await invoke<string>('evm_create_mnemonic');

      const response = await invoke<CreateWalletResponse>('evm_create_wallet_from_mnemonic', {
        mnemonicPhrase: mnemonic,
        walletLabel: walletLabel || undefined,
      });

      setGeneratedMnemonic(response);
      setShowMnemonicDialog(true);
      setMnemonicInput('');
      setWalletLabel('');

      loadWallets();
    } catch (error) {
      console.error('Error creating wallet:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleImportMnemonic = async () => {
    if (!mnemonicInput.trim()) return;

    setIsLoading(true);
    try {
      const response = await invoke<CreateWalletResponse>('evm_create_wallet_from_mnemonic', {
        mnemonicPhrase: mnemonicInput,
        walletLabel: walletLabel || undefined,
      });

      setWallets([...wallets, response.wallet]);
      setMnemonicInput('');
      setWalletLabel('');
      setIsDialogOpen(false);
      loadWallets();
    } catch (error) {
      console.error('Error importing mnemonic:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleImportPrivateKey = async () => {
    if (!privateKeyInput.trim()) return;

    setIsLoading(true);
    try {
      const response = await invoke<CreateWalletResponse>('evm_create_wallet_from_private_key', {
        privateKey: privateKeyInput,
        walletLabel: walletLabel || undefined,
      });

      setWallets([...wallets, response.wallet]);
      setPrivateKeyInput('');
      setWalletLabel('');
      setIsDialogOpen(false);
      loadWallets();
    } catch (error) {
      console.error('Error importing private key:', error);
      alert(`Error: ${error}`);
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
    setShowMnemonicDialog(false);
    setGeneratedMnemonic(null);
    setIsDialogOpen(false);
  };

  const handleExportPrivateKey = async (walletId: string) => {
    setIsLoading(true);
    try {
      const secret = await invoke<string>('evm_export_private_key', { walletId });
      setExportedSecret(secret);
      setExportedSecretType('private-key');
      setShowExportDialog(true);
    } catch (error) {
      console.error('Error exporting private key:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleExportMnemonic = async (walletId: string) => {
    setIsLoading(true);
    try {
      const secret = await invoke<string>('evm_export_mnemonic', { walletId });
      setExportedSecret(secret);
      setExportedSecretType('mnemonic');
      setShowExportDialog(true);
    } catch (error) {
      console.error('Error exporting mnemonic:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleDeleteWallet = async (walletId: string) => {
    setIsLoading(true);
    try {
      await invoke<boolean>('evm_delete_wallet', { walletId });
      setWallets(wallets.filter(w => w.id !== walletId));
      setDeleteConfirm(null);
      setWalletsWithBalances(prev => {
        const newMap = new Map(prev);
        newMap.delete(walletId);
        return newMap;
      });
    } catch (error) {
      console.error('Error deleting wallet:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSendEvm = async () => {
    if (!selectedWalletForSend || !selectedAssetForSend || !sendToAddress || !sendAmount) return;

    setIsSending(true);
    try {
      // Amount in the backend is expected in "wei" for eth_sendTransaction but 
      // let's check what SendEvmRequest expects.
      // SendEvmRequest amount is "in token units (e.g., '1.5' for 1.5 ETH)"
      // Actually it's stored as String. 
      // But wait, look at send_evm_transaction implementation:
      // let amount_wei = U256::from_dec_str(&request.amount)
      // This means the backend EXPECTS WEI if it uses from_dec_str on the string.
      // Wait, let me re-check transaction.rs line 317:
      // let amount_wei = U256::from_dec_str(&request.amount)
      // Yes, it expects the raw integer value (wei) as a decimal string.

      const decimals = selectedAssetForSend.asset.asset.decimals;
      const amountFloat = parseFloat(sendAmount);
      // Simple conversion to wei-like string (might lose precision for very small amounts, but okay for basic UI)
      const amountInBaseUnits = BigInt(Math.floor(amountFloat * Math.pow(10, decimals))).toString();

      const response = await invoke<{ tx_hash: string; message: string }>('send_evm', {
        request: {
          wallet_id: selectedWalletForSend.id,
          to_address: sendToAddress,
          amount: amountInBaseUnits,
          chain: selectedAssetForSend.chain,
          chain_id: selectedAssetForSend.chainId,
          asset_symbol: selectedAssetForSend.asset.asset.symbol,
          contract_address: selectedAssetForSend.asset.asset.contract_address,
          gas_limit: estimatedGasLimit,
          gas_price: estimatedGasPrice,
        }
      });

      toast.success(
        <div className="flex flex-col gap-1">
          <div className="font-semibold">Transaction Sent Successfully</div>
          <div className="text-xs font-mono text-muted-foreground break-all">
            {response.tx_hash}
          </div>
          <Button
            variant="link"
            size="sm"
            className="p-0 h-auto text-indigo-400 hover:text-indigo-300 justify-start"
            onClick={() => openExternalLink(getEvmExplorerUrl(response.tx_hash, selectedAssetForSend.chainId))}
          >
            <ExternalLink className="w-3 h-3 mr-1" />
            View on Explorer
          </Button>
        </div>,
        {
          duration: 10000,
        }
      );

      setIsSendDialogOpen(false);
      setSendToAddress('');
      setSendAmount('');

      // Refresh balance after successful send
      handleRefreshBalance(selectedWalletForSend.id);
    } catch (error) {
      console.error('Error sending EVM asset:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsSending(false);
    }
  };

  const getWalletMainnetBalance = (wallet: EvmWalletInfo) => {
    return wallet.chains
      .filter(chain => chain.chain_id !== 11155111) // Exclude Sepolia
      .reduce((sum, chain) => sum + chain.total_balance_usd, 0);
  };

  const totalBalance = Array.from(walletsWithBalances.values()).reduce(
    (sum, wallet) => sum + getWalletMainnetBalance(wallet),
    0
  );

  return (
    <Card className="p-6 select-none glass-card">
      <div className="space-y-6">
        {/* Header with Refresh Button */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="p-3 bg-gradient-to-br from-blue-400 via-indigo-500 to-purple-600 rounded-2xl shadow-lg shadow-indigo-500/20">
              <img src="/images/assets/ethereum.png" alt="Ethereum" className="w-8 h-8" />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-foreground">EVM</h3>
              <p className="text-xs text-muted-foreground font-medium">{wallets.length} wallet{wallets.length !== 1 ? 's' : ''}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button
              onClick={() => handleRefreshBalance()}
              variant="outline"
              className="gap-2"
              disabled={isRefreshing || wallets.length === 0}
            >
              {isRefreshing ? (
                <>
                  <div className="w-4 h-4 border-2 border-muted-foreground/30 border-t-gray-900 rounded-full animate-spin" />
                  Refreshing...
                </>
              ) : (
                <>
                  ↻
                  Refresh All
                </>
              )}
            </Button>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button className="gap-2">
                  <Plus className="w-4 h-4" />
                  Add / Import Wallet
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>EVM Wallet Management</DialogTitle>
                </DialogHeader>
                <Tabs defaultValue="create" className="w-full">
                  <TabsList className="grid w-full grid-cols-3">
                    <TabsTrigger value="create">Create New</TabsTrigger>
                    <TabsTrigger value="mnemonic">Import Mnemonic</TabsTrigger>
                    <TabsTrigger value="private-key">Import Private Key</TabsTrigger>
                  </TabsList>

                  {/* Create New Wallet */}
                  <TabsContent value="create" className="space-y-4 mt-4">
                    <p className="text-sm text-muted-foreground">
                      Create a new wallet with a secure mnemonic phrase. This wallet will work across all supported EVM chains (Ethereum, Arbitrum, etc).
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
                        placeholder="Enter your private key (hex format, with or without 0x prefix)..."
                        value={privateKeyInput}
                        onChange={(e) => setPrivateKeyInput(e.target.value)}
                        rows={3}
                        className="font-mono text-sm"
                      />
                      <p className="text-xs text-muted-foreground">
                        Support 64-character hex string (with or without 0x prefix)
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
        </div>

        {/* Mnemonic Display Dialog */}
        <Dialog open={showMnemonicDialog} onOpenChange={setShowMnemonicDialog}>
          <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
            <DialogHeader>
              <DialogTitle>Save Your Mnemonic Phrase</DialogTitle>
            </DialogHeader>

            {generatedMnemonic && (
              <div className="space-y-6">
                {/* Warning Alert */}
                <div className="bg-red-50 border border-red-200 rounded-lg p-4 space-y-2">
                  <div className="flex items-start gap-3">
                    <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
                    <div className="space-y-2">
                      <p className="font-semibold text-red-900">Important: This is your only chance to save this mnemonic phrase!</p>
                      <p className="text-sm text-red-800">
                        Your mnemonic phrase is not stored on this device. If you close this dialog without saving it, you will lose access to this wallet forever.
                      </p>
                      <p className="text-sm text-red-800">
                        • Never share your mnemonic phrase with anyone<br />
                        • Store it in a safe location<br />
                        • Anyone with this phrase can access your funds
                      </p>
                    </div>
                  </div>
                </div>

                {/* Wallet Info */}
                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                  <div className="space-y-2">
                    <div>
                      <p className="text-sm text-muted-foreground">Wallet Label</p>
                      <p className="font-medium text-white">{generatedMnemonic.wallet.label}</p>
                    </div>
                    <div>
                      <p className="text-sm text-muted-foreground">Wallet Address (Ethereum)</p>
                      <div className="flex items-center gap-2 mt-1">
                        <p className="font-mono text-sm text-white break-all">{generatedMnemonic.wallet.address}</p>
                        <button
                          onClick={() => handleCopyAddress(generatedMnemonic.wallet.address)}
                          className="text-gray-400 hover:text-muted-foreground flex-shrink-0 transition-colors"
                          title="Copy address"
                        >
                          {addressCopied === generatedMnemonic.wallet.address ? (
                            <CheckCircle2 className="w-4 h-4 text-green-600" />
                          ) : (
                            <Copy className="w-4 h-4" />
                          )}
                        </button>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Mnemonic Phrase */}
                <div className="space-y-2">
                  <Label>Your Mnemonic Phrase</Label>
                  <div className="bg-muted rounded-lg p-4 space-y-3">
                    <div className="grid grid-cols-3 gap-2">
                      {generatedMnemonic.mnemonic.split(' ').map((word, index) => (
                        <div key={index} className="flex items-center gap-2">
                          <span className="text-muted-foreground text-xs font-mono w-6">{index + 1}.</span>
                          <span className="text-yellow-400 font-mono text-sm">{word}</span>
                        </div>
                      ))}
                    </div>
                    <button
                      onClick={() => handleCopyMnemonic(generatedMnemonic.mnemonic)}
                      className="w-full mt-2 px-3 py-2 bg-muted/80 hover:bg-muted/70 text-gray-200 rounded text-sm transition-colors flex items-center justify-center gap-2"
                    >
                      {mnemonicCopied ? (
                        <>
                          <CheckCircle2 className="w-4 h-4" />
                          Copied!
                        </>
                      ) : (
                        <>
                          <Copy className="w-4 h-4" />
                          Copy All
                        </>
                      )}
                    </button>
                  </div>
                </div>

                {/* Save Instructions */}
                <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4 space-y-2">
                  <p className="font-semibold text-yellow-900 text-sm">How to save your mnemonic:</p>
                  <ul className="text-sm text-yellow-800 space-y-1 list-disc list-inside">
                    <li>Write it down on paper and store it in a safe place</li>
                    <li>Use a password manager to securely store it</li>
                    <li>Do NOT store it in plain text files or screenshots</li>
                    <li>Do NOT store it in cloud services or emails</li>
                  </ul>
                </div>

                {/* Action Buttons */}
                <div className="flex gap-3">
                  <Button
                    variant="outline"
                    className="flex-1"
                    onClick={() => handleCopyMnemonic(generatedMnemonic.mnemonic)}
                  >
                    Copy to Clipboard
                  </Button>
                  <Button
                    className="flex-1 bg-green-600 hover:bg-green-700"
                    onClick={handleCloseMnemonicDialog}
                  >
                    I've Saved My Mnemonic
                  </Button>
                </div>
              </div>
            )}
          </DialogContent>
        </Dialog>

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

        {/* Send EVM Dialog */}
        <Dialog open={isSendDialogOpen} onOpenChange={setIsSendDialogOpen}>
          <DialogContent className="sm:max-w-[480px] p-0 border-none shadow-2xl bg-[#1A1B23]">
            <DialogHeader className="p-6 pb-2">
              <DialogTitle className="text-xl font-bold flex items-center justify-between">
                <span>Send</span>
                <span className="text-sm font-normal text-muted-foreground mr-8">Cancel</span>
              </DialogTitle>
            </DialogHeader>

            <div className="px-6 pb-6 space-y-6">
              {/* Asset Selector Display (MetaMask style) */}
              <div className="space-y-2">
                <div className="flex justify-between items-end">
                  <Label className="text-xs text-muted-foreground uppercase font-bold tracking-wider">Asset</Label>
                  <span className="text-[11px] text-muted-foreground">
                    Balance: <span className="text-foreground font-mono">{selectedAssetForSend?.asset.balance_float.toFixed(6)} {selectedAssetForSend?.asset.asset.symbol}</span>
                  </span>
                </div>
                <div className="flex items-center justify-between p-4 bg-[#252833] rounded-xl border border-white/5 cursor-pointer hover:bg-[#2C303D] transition-colors">
                  <div className="flex items-center gap-3">
                    <div className="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-xs font-bold shadow-lg">
                      {selectedAssetForSend?.asset.asset.symbol[0]}
                    </div>
                    <div>
                      <p className="text-sm font-bold text-white">{selectedAssetForSend?.asset.asset.symbol}</p>
                      <p className="text-xs text-muted-foreground">on {selectedAssetForSend?.chain}</p>
                    </div>
                  </div>
                  <ChevronRight className="w-4 h-4 text-muted-foreground" />
                </div>
              </div>

              {/* To Address Input */}
              <div className="space-y-2">
                <Label htmlFor="to-address" className="text-xs text-muted-foreground uppercase font-bold tracking-wider">To</Label>
                <div className="relative">
                  <Input
                    id="to-address"
                    placeholder="Search, public address (0x), or ENS"
                    value={sendToAddress}
                    onChange={(e) => setSendToAddress(e.target.value)}
                    className="h-12 bg-[#252833] border-white/5 rounded-xl pr-10 focus:ring-indigo-500/20 focus:border-indigo-500/50"
                  />
                  {sendToAddress && (
                    <button
                      onClick={() => setSendToAddress('')}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-white"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>

              {/* Amount Input with Max button */}
              <div className="space-y-2">
                <div className="flex justify-between items-center">
                  <Label htmlFor="amount" className="text-xs text-muted-foreground uppercase font-bold tracking-wider">Amount</Label>
                  <button
                    onClick={() => setSendAmount(selectedAssetForSend?.asset.balance_float.toString() || '0')}
                    className="text-[11px] text-indigo-400 hover:text-indigo-300 font-bold uppercase tracking-tight"
                  >
                    Max
                  </button>
                </div>
                <div className="relative group">
                  <Input
                    id="amount"
                    type="number"
                    placeholder="0"
                    value={sendAmount}
                    onChange={(e) => setSendAmount(e.target.value)}
                    className="h-14 bg-[#252833] border-white/5 rounded-xl text-2xl font-bold pr-16 focus:ring-indigo-500/20 focus:border-indigo-500/50"
                  />
                  <span className="absolute right-4 top-1/2 -translate-y-1/2 text-lg font-bold text-muted-foreground group-focus-within:text-indigo-400 transition-colors">
                    {selectedAssetForSend?.asset.asset.symbol}
                  </span>
                </div>
                {selectedAssetForSend?.asset.usd_price && sendAmount && (
                  <p className="text-xs text-muted-foreground text-right mt-1">
                    ≈ ${(parseFloat(sendAmount) * selectedAssetForSend.asset.usd_price).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                  </p>
                )}
              </div>

              {/* Quote/Gas Estimation */}
              <div className="p-4 bg-indigo-500/5 rounded-xl border border-indigo-500/10 space-y-3">
                <div className="flex justify-between items-center">
                  <div className="flex items-center gap-1.5">
                    <span className="text-xs text-muted-foreground">Estimated gas fee</span>
                    <HelpCircle className="w-3.5 h-3.5 text-muted-foreground" />
                  </div>
                  <div className="text-right">
                    {isEstimatingGas ? (
                      <div className="w-3 h-3 border border-indigo-400 border-t-transparent rounded-full animate-spin ml-auto" />
                    ) : gasEstimationError ? (
                      <p className="text-[10px] text-red-400">{gasEstimationError}</p>
                    ) : estimatedGasLimit && estimatedGasPrice ? (
                      <>
                        <p className="text-xs font-bold text-white">
                          {(() => {
                            const feeWei = BigInt(estimatedGasLimit) * BigInt(estimatedGasPrice);
                            const feeEth = Number(feeWei) / 1e18;
                            return `${feeEth.toFixed(6)} ETH`;
                          })()}
                        </p>
                        {selectedAssetForSend?.asset.usd_price && (
                          <p className="text-[10px] text-muted-foreground">
                            ≈ ${((Number(BigInt(estimatedGasLimit) * BigInt(estimatedGasPrice)) / 1e18) * (walletsWithBalances.get(selectedWalletForSend?.id || '')?.chains.find(c => c.chain_id === selectedAssetForSend.chainId)?.assets.find(a => a.asset.contract_address === null)?.usd_price || 0)).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                          </p>
                        )}
                      </>
                    ) : (
                      <p className="text-xs font-bold text-white">--</p>
                    )}
                  </div>
                </div>
                <div className="h-px bg-white/5" />
                <div className="flex justify-between items-center">
                  <span className="text-sm font-bold text-white">Total</span>
                  <div className="text-right">
                    <p className="text-sm font-bold text-indigo-400">
                      {(() => {
                        const amount = parseFloat(sendAmount) || 0;
                        const feeEth = estimatedGasLimit && estimatedGasPrice
                          ? Number(BigInt(estimatedGasLimit) * BigInt(estimatedGasPrice)) / 1e18
                          : 0;

                        if (selectedAssetForSend?.asset.asset.contract_address === null) {
                          // Native token: Amount + Fee
                          return `${(amount + feeEth).toFixed(6)} ${selectedAssetForSend.asset.asset.symbol}`;
                        } else {
                          // ERC20: Amount + Fee (in Native)
                          return `${amount.toFixed(6)} ${selectedAssetForSend?.asset.asset.symbol} + ${feeEth.toFixed(6)} ETH`;
                        }
                      })()}
                    </p>
                    <p className="text-[10px] text-muted-foreground">Amount + gas fee</p>
                  </div>
                </div>
              </div>

              <div className="flex gap-4 pt-2">
                <Button
                  variant="outline"
                  className="flex-1 h-12 rounded-full border-indigo-500/30 text-indigo-400 hover:bg-indigo-500/10"
                  onClick={() => setIsSendDialogOpen(false)}
                >
                  Reject
                </Button>
                <Button
                  className="flex-1 h-12 rounded-full bg-indigo-600 hover:bg-indigo-700 text-white font-bold shadow-lg shadow-indigo-600/20"
                  onClick={handleSendEvm}
                  disabled={isSending || !sendToAddress || !sendAmount || parseFloat(sendAmount) <= 0}
                >
                  {isSending ? (
                    <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  ) : (
                    "Confirm"
                  )}
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>

        {/* Total Balance */}
        {wallets.length > 0 && (
          <div className="bg-muted/30 rounded-lg p-4 border border-border/50">
            <p className="text-xs text-muted-foreground font-medium mb-2 uppercase tracking-wide">Total Balance (All Chains)</p>
            <div className="flex items-baseline gap-3">
              <p className="text-3xl font-light tracking-tight text-foreground font-mono">
                ${totalBalance.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </p>
            </div>
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
              const walletWithBalances = walletsWithBalances.get(wallet.id);
              const lastRefresh = lastRefreshTime.get(wallet.id);
              return (
                <div
                  key={wallet.id}
                  className="border border-border rounded-lg p-4 hover:bg-muted/30 transition-colors space-y-4"
                >
                  {/* Wallet Header */}
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
                          ${(walletWithBalances ? getWalletMainnetBalance(walletWithBalances) : 0).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </p>
                        <p className="text-xs text-muted-foreground mt-1">
                          {lastRefresh ? `Updated: ${lastRefresh.toLocaleTimeString()}` : 'Loading...'}
                        </p>
                      </div>
                      {/* Action Buttons */}
                      <div className="flex gap-1 flex-wrap justify-end">
                        {/* Refresh Button */}
                        <button
                          onClick={() => handleRefreshBalance(wallet.id)}
                          className="px-2 py-1 text-xs bg-muted/50 text-muted-foreground hover:bg-muted hover:text-foreground rounded transition-colors flex items-center gap-1"
                          title="Refresh balance from blockchain"
                          disabled={isRefreshing || refreshingWalletId === wallet.id}
                        >
                          {refreshingWalletId === wallet.id ? (
                            <div className="w-3 h-3 border border-muted-foreground/60 border-t-transparent rounded-full animate-spin" />
                          ) : (
                            <span>↻</span>
                          )}
                          {refreshingWalletId === wallet.id ? 'Refreshing' : 'Refresh'}
                        </button>

                        {/* Export Private Key Button */}
                        <button
                          onClick={() => handleExportPrivateKey(wallet.id)}
                          className="px-2 py-1 text-xs bg-blue-50 text-blue-700 hover:bg-blue-100 rounded transition-colors flex items-center gap-1"
                          title="Export private key"
                          disabled={isLoading}
                        >
                          <Download className="w-3 h-3" />
                          <span>Private Key</span>
                        </button>

                        {/* Export Mnemonic Button (only for mnemonic wallets) */}
                        {wallet.wallet_type === 'mnemonic' && (
                          <button
                            onClick={() => handleExportMnemonic(wallet.id)}
                            className="px-2 py-1 text-xs bg-green-50 text-green-700 hover:bg-green-100 rounded transition-colors flex items-center gap-1"
                            title="Export mnemonic phrase"
                            disabled={isLoading}
                          >
                            <Download className="w-3 h-3" />
                            <span>Mnemonic</span>
                          </button>
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

                  {/* Chain Assets */}
                  {walletWithBalances && walletWithBalances.chains.length > 0 && (
                    <div className="border-t border-border pt-4 space-y-2">
                      <p className="text-xs font-semibold text-foreground uppercase tracking-wide">Assets by Chain</p>
                      <Accordion type="single" collapsible className="w-full">
                        {walletWithBalances.chains.map((chainAssets, chainIndex) => (
                          <AccordionItem key={chainIndex} value={`chain-${chainAssets.chain}`}>
                            <AccordionTrigger className="hover:no-underline py-2">
                              <div className="flex items-center gap-3 flex-1 text-left">
                                <div className="flex-1">
                                  <p className="font-medium text-foreground capitalize text-sm">{chainAssets.chain}</p>
                                  <p className="text-xs text-muted-foreground">Chain ID: {chainAssets.chain_id}</p>
                                </div>
                                <div className="text-right">
                                  <p className="font-semibold text-foreground text-sm font-mono">${chainAssets.total_balance_usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</p>
                                </div>
                              </div>
                            </AccordionTrigger>
                            <AccordionContent>
                              <div className="space-y-2 pt-2">
                                {chainAssets.assets.map((assetBalance, assetIndex) => (
                                  <div key={assetIndex} className="flex items-center justify-between px-2 py-2 bg-muted/30 rounded group">
                                    <div className="flex items-center gap-3">
                                      <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-xs font-semibold">
                                        {assetBalance.asset.symbol[0]}
                                      </div>
                                      <div>
                                        <p className="font-medium text-foreground text-sm">{assetBalance.asset.symbol}</p>
                                        <p className="text-xs text-muted-foreground">{assetBalance.asset.name}</p>
                                      </div>
                                    </div>
                                    <div className="flex items-center gap-4">
                                      <div className="text-right">
                                        <p className="font-mono text-xs text-foreground">{assetBalance.balance_float.toFixed(6)} {assetBalance.asset.symbol}</p>
                                        {assetBalance.usd_value > 0 && (
                                          <p className="text-xs text-muted-foreground mt-1 font-mono">
                                            ${assetBalance.usd_value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                                          </p>
                                        )}
                                      </div>
                                      <button
                                        onClick={() => {
                                          setSelectedWalletForSend(wallet);
                                          setSelectedAssetForSend({
                                            chain: chainAssets.chain,
                                            chainId: chainAssets.chain_id,
                                            asset: assetBalance
                                          });
                                          setIsSendDialogOpen(true);
                                        }}
                                        className="p-2 bg-indigo-500/10 text-indigo-400 hover:bg-indigo-500 hover:text-white rounded-full transition-all opacity-0 group-hover:opacity-100 shadow-sm"
                                        title="Send"
                                      >
                                        <Send className="w-4 h-4" />
                                      </button>
                                    </div>
                                  </div>
                                ))}
                              </div>
                            </AccordionContent>
                          </AccordionItem>
                        ))}
                      </Accordion>
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
    </Card>
  );
};

export default EvmAssets;
