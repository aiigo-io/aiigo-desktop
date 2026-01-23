import React, { useState, useEffect } from 'react';
import { Card, Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger, Tabs, TabsContent, TabsList, TabsTrigger, Label, Textarea, Input, Badge } from '@/components/ui';
import { Copy, Plus, AlertCircle, CheckCircle2, Trash2, Download, RefreshCw, Send, ExternalLink, HelpCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
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
  mnemonic: string;
  wallet: WalletInfo;
}

const BitcoinAssets: React.FC = () => {
  const [wallets, setWallets] = useState<WalletInfo[]>([]);
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
  const [refreshingBalance, setRefreshingBalance] = useState<string | null>(null);
  const [btcPrice, setBtcPrice] = useState<number>(0);

  // Send BTC State
  const [isSendDialogOpen, setIsSendDialogOpen] = useState(false);
  const [selectedWalletForSend, setSelectedWalletForSend] = useState<WalletInfo | null>(null);
  const [sendToAddress, setSendToAddress] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [sendFeeRate, setSendFeeRate] = useState<number>(1);
  const [isSending, setIsSending] = useState(false);

  // Fee Estimation State
  const [isEstimatingFees, setIsEstimatingFees] = useState(false);
  const [estimatedFees, setEstimatedFees] = useState<{ fast: number; half_hour: number; hour: number } | null>(null);
  const [feeRateType, setFeeRateType] = useState<'fast' | 'half_hour' | 'hour' | 'custom'>('half_hour');
  const [isSendAll, setIsSendAll] = useState(false);

  // Load wallets on mount
  useEffect(() => {
    loadWallets();
    fetchBtcPrice();

    // Refresh BTC price every 60 seconds
    const priceInterval = setInterval(fetchBtcPrice, 60000);

    return () => clearInterval(priceInterval);
  }, []);

  const loadWallets = async () => {
    try {
      const result = await invoke<WalletInfo[]>('bitcoin_get_wallets');
      setWallets(result);
    } catch (error) {
      console.error('Error loading wallets:', error);
    }
  };

  const fetchFeeEstimates = async () => {
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
    try {
      const price = await invoke<number>('get_bitcoin_price');
      setBtcPrice(price);
    } catch (error) {
      console.error('Error fetching BTC price from backend:', error);
      // Fallback to a reasonable default if API fails
      if (btcPrice === 0) {
        setBtcPrice(95000); // Reasonable fallback
      }
    }
  };

  const handleCreateMnemonic = async () => {
    setIsLoading(true);
    try {
      // Generate mnemonic
      const mnemonic = await invoke<string>('bitcoin_create_mnemonic');

      // Create wallet from mnemonic
      const response = await invoke<CreateWalletResponse>('bitcoin_create_wallet_from_mnemonic', {
        mnemonicPhrase: mnemonic,
        walletLabel: walletLabel || undefined,
      });

      setGeneratedMnemonic(response);
      setShowMnemonicDialog(true);
      setMnemonicInput('');
      setWalletLabel('');

      // Reload wallets
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
      // Import and create wallet from mnemonic
      const response = await invoke<CreateWalletResponse>('bitcoin_create_wallet_from_mnemonic', {
        mnemonicPhrase: mnemonicInput,
        walletLabel: walletLabel || undefined,
      });

      setWallets([...wallets, response.wallet]);
      setMnemonicInput('');
      setWalletLabel('');
      setIsDialogOpen(false);
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
      // Import and create wallet from private key
      const response = await invoke<CreateWalletResponse>('bitcoin_create_wallet_from_private_key', {
        privateKey: privateKeyInput,
        walletLabel: walletLabel || undefined,
      });

      setWallets([...wallets, response.wallet]);
      setPrivateKeyInput('');
      setWalletLabel('');
      setIsDialogOpen(false);
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
      const secret = await invoke<string>('bitcoin_export_private_key', { walletId });
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
      const secret = await invoke<string>('bitcoin_export_mnemonic', { walletId });
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
      await invoke<boolean>('bitcoin_delete_wallet', { walletId });
      setWallets(wallets.filter(w => w.id !== walletId));
      setDeleteConfirm(null);
    } catch (error) {
      console.error('Error deleting wallet:', error);
      alert(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRefreshBalance = async (walletId: string) => {
    setRefreshingBalance(walletId);
    try {
      const updatedWallet = await invoke<WalletInfo>('bitcoin_get_wallet_with_balance', { walletId });
      setWallets(wallets.map(w => w.id === walletId ? updatedWallet : w));
    } catch (error) {
      console.error('Error refreshing balance:', error);
      alert(`Error refreshing balance: ${error}`);
    } finally {
      setRefreshingBalance(null);
    }
  };

  const handleSendBtc = async () => {
    if (!selectedWalletForSend || !sendToAddress || !sendAmount) return;

    setIsSending(true);
    try {
      const response = await invoke<{ tx_hash: string; message: string }>('send_bitcoin', {
        request: {
          wallet_id: selectedWalletForSend.id,
          to_address: sendToAddress,
          amount: parseFloat(sendAmount),
          fee_rate: sendFeeRate,
          send_all: isSendAll,
        }
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

      setIsSendDialogOpen(false);
      setSendToAddress('');
      setSendAmount('');

      // Refresh balance after successful send
      handleRefreshBalance(selectedWalletForSend.id);
    } catch (error) {
      console.error('Error sending BTC:', error);
      toast.error(`Error: ${error}`);
    } finally {
      setIsSending(false);
    }
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
          <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
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
                      <p className="text-sm text-muted-foreground">Wallet Address</p>
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

        {/* Send Bitcoin Dialog */}
        <Dialog open={isSendDialogOpen} onOpenChange={setIsSendDialogOpen}>
          <DialogContent className="sm:max-w-[450px]">
            <DialogHeader>
              <DialogTitle>Send Bitcoin</DialogTitle>
            </DialogHeader>
            <div className="space-y-4 py-4">
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
                  {btcPrice > 0 && sendAmount && !isNaN(parseFloat(sendAmount)) && (
                    <span className="text-[10px] text-muted-foreground font-mono">
                      ≈ ${(parseFloat(sendAmount) * btcPrice).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
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
                      {btcPrice > 0 && (
                        <p className="text-[10px] text-muted-foreground">
                          ≈ ${((148 * sendFeeRate) / 100_000_000 * btcPrice).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
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
            </div>
            <div className="flex gap-3">
              <Button variant="outline" className="flex-1" onClick={() => setIsSendDialogOpen(false)}>
                Cancel
              </Button>
              <Button
                className="flex-1 bg-orange-600 hover:bg-orange-700 shadow-lg shadow-orange-500/20"
                onClick={handleSendBtc}
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
                    Send Bitcoin
                  </>
                )}
              </Button>
            </div>
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
              {btcPrice > 0 && (
                <p className="text-sm text-muted-foreground font-mono">
                  ${(totalBalance * btcPrice).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </p>
              )}
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
            wallets.map((wallet) => (
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
                      {btcPrice > 0 && (
                        <p className="text-xs text-muted-foreground mt-1 font-mono">
                          ${(wallet.balance * btcPrice).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </p>
                      )}
                    </div>
                    {/* Action Buttons */}
                    <div className="flex gap-1 flex-wrap justify-end">
                      {/* Send Button */}
                      <button
                        onClick={() => {
                          setSelectedWalletForSend(wallet);
                          setIsSendDialogOpen(true);
                        }}
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
              </div>
            ))
          )}
        </div>
      </div>
    </Card>
  );
};

export default BitcoinAssets;