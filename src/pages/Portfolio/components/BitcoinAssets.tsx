import React, { useState, useEffect } from 'react';
import { Card, Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger, Tabs, TabsContent, TabsList, TabsTrigger, Label, Textarea, Input, Badge } from '@/components/ui';
import { Copy, Plus, AlertCircle, CheckCircle2, Trash2, Download } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { shortAddress } from '@/lib/utils';

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

  // Load wallets on mount
  useEffect(() => {
    loadWallets();
  }, []);

  const loadWallets = async () => {
    try {
      const result = await invoke<WalletInfo[]>('bitcoin_get_wallets');
      setWallets(result);
    } catch (error) {
      console.error('Error loading wallets:', error);
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

  const totalBalance = wallets.reduce((sum, wallet) => sum + wallet.balance, 0);

  return (
    <Card className="p-6 select-none">
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <img src="/images/assets/bitcoin.png" alt="Bitcoin" className="w-8 h-8" />
            <div>
              <h3 className="text-2xl font-semibold">Bitcoin</h3>
              <p className="text-sm text-gray-500">{wallets.length} wallet{wallets.length !== 1 ? 's' : ''}</p>
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
                  <p className="text-sm text-gray-600">
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
                    <p className="text-xs text-gray-500">
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
                    <p className="text-xs text-gray-500">
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
                        • Never share your mnemonic phrase with anyone<br/>
                        • Store it in a safe location<br/>
                        • Anyone with this phrase can access your funds
                      </p>
                    </div>
                  </div>
                </div>

                {/* Wallet Info */}
                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                  <div className="space-y-2">
                    <div>
                      <p className="text-sm text-gray-600">Wallet Label</p>
                      <p className="font-medium text-gray-900">{generatedMnemonic.wallet.label}</p>
                    </div>
                    <div>
                      <p className="text-sm text-gray-600">Wallet Address</p>
                      <div className="flex items-center gap-2 mt-1">
                        <p className="font-mono text-sm text-gray-900 break-all">{generatedMnemonic.wallet.address}</p>
                        <button
                          onClick={() => handleCopyAddress(generatedMnemonic.wallet.address)}
                          className="text-gray-400 hover:text-gray-600 flex-shrink-0 transition-colors"
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
                  <div className="bg-gray-900 rounded-lg p-4 space-y-3">
                    <div className="grid grid-cols-3 gap-2">
                      {generatedMnemonic.mnemonic.split(' ').map((word, index) => (
                        <div key={index} className="flex items-center gap-2">
                          <span className="text-gray-500 text-xs font-mono w-6">{index + 1}.</span>
                          <span className="text-yellow-400 font-mono text-sm">{word}</span>
                        </div>
                      ))}
                    </div>
                    <button
                      onClick={() => handleCopyMnemonic(generatedMnemonic.mnemonic)}
                      className="w-full mt-2 px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-200 rounded text-sm transition-colors flex items-center justify-center gap-2"
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
                  <div className="bg-gray-900 rounded-lg p-4">
                    {exportedSecretType === 'mnemonic' ? (
                      <div className="grid grid-cols-3 gap-2 mb-3">
                        {exportedSecret.split(' ').map((word, index) => (
                          <div key={index} className="flex items-center gap-2">
                            <span className="text-gray-500 text-xs font-mono w-6">{index + 1}.</span>
                            <span className="text-yellow-400 font-mono text-sm break-all">{word}</span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p className="text-yellow-400 font-mono text-sm break-all">{exportedSecret}</p>
                    )}
                    <button
                      onClick={() => handleCopyExportedSecret(exportedSecret)}
                      className="w-full mt-3 px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-200 rounded text-sm transition-colors flex items-center justify-center gap-2"
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

        {/* Total Balance */}
        {wallets.length > 0 && (
          <div className="bg-gradient-to-br from-orange-50 to-yellow-50 rounded-lg p-4 border border-orange-200">
            <p className="text-sm text-gray-600 mb-1">Total Balance</p>
            <div className="flex items-baseline gap-2">
              <p className="text-3xl font-bold text-gray-900">{totalBalance.toFixed(4)} BTC</p>
              <p className="text-gray-600">
                ${(totalBalance * 42000).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </p>
            </div>
          </div>
        )}

        {/* Wallets List */}
        <div className="space-y-3">
          {wallets.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-gray-500 mb-4">No wallets yet</p>
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
                className="border rounded-lg p-4 hover:bg-gray-50 transition-colors space-y-3"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-gray-900">
                        {wallet.label}
                      </p>
                      <Badge variant="secondary" className="text-xs">
                        {wallet.wallet_type === 'mnemonic' ? 'Mnemonic' : 'Private Key'}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2 mt-2">
                      <p className="font-mono text-sm text-gray-600 truncate">
                        {shortAddress(wallet.address)}
                      </p>
                      <button
                        onClick={() => handleCopyAddress(wallet.address)}
                        className="text-gray-400 hover:text-gray-600 transition-colors flex-shrink-0"
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
                      <p className="font-semibold text-lg text-gray-900">
                        {wallet.balance.toFixed(4)} BTC
                      </p>
                      <p className="text-sm text-gray-500 mt-1">
                        ${(wallet.balance * 42000).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                      </p>
                    </div>
                    {/* Action Buttons */}
                    <div className="flex gap-1 flex-wrap justify-end">
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
                            className="px-2 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded"
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