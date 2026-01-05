import React, { useState, useEffect } from 'react';
import { Card } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Wallet, Plus } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { shortAddress } from '@/lib/utils';
import { Button } from '@/components/ui/button';

interface WalletInfo {
  id: string;
  label: string;
  wallet_type: 'mnemonic' | 'private-key';
  address: string;
  balance: number;
  created_at: string;
  updated_at: string;
}

interface WalletSelectorProps {
  onWalletChange?: (wallet: WalletInfo | null) => void;
}

export const WalletSelector: React.FC<WalletSelectorProps> = ({ onWalletChange }) => {
  const [wallets, setWallets] = useState<WalletInfo[]>([]);
  const [selectedWallet, setSelectedWallet] = useState<WalletInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadWallets();
  }, []);

  const loadWallets = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<WalletInfo[]>('evm_get_wallets');
      setWallets(result);

      // Auto-select first wallet if available
      if (result.length > 0 && !selectedWallet) {
        setSelectedWallet(result[0]);
        onWalletChange?.(result[0]);
      }
    } catch (error) {
      console.error('Error loading wallets:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleWalletChange = (walletId: string) => {
    const wallet = wallets.find(w => w.id === walletId) || null;
    setSelectedWallet(wallet);
    onWalletChange?.(wallet);
  };

  if (isLoading) {
    return (
      <Card className="p-4 bg-muted/30 backdrop-blur-sm border-border/50">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Wallet className="size-4" />
          <span className="text-sm">Loading wallets...</span>
        </div>
      </Card>
    );
  }

  if (wallets.length === 0) {
    return (
      <Card className="p-4 bg-muted/30 backdrop-blur-sm border-border/50">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Wallet className="size-4" />
            <span className="text-sm">No wallets found</span>
          </div>
          <Button
            size="sm"
            variant="outline"
            className="gap-2"
            onClick={() => {
              // Navigate to portfolio page to create wallet
              window.location.href = '/portfolio';
            }}
          >
            <Plus className="size-3" />
            Add Wallet
          </Button>
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-4 bg-muted/30 backdrop-blur-sm border-border/50">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2 text-foreground">
          <Wallet className="size-4" />
          <span className="text-sm font-medium">Wallet:</span>
        </div>

        <Select
          value={selectedWallet?.id || ''}
          onValueChange={handleWalletChange}
        >
          <SelectTrigger className="flex-1 h-9 bg-card border-border">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {wallets.map((wallet) => (
              <SelectItem key={wallet.id} value={wallet.id}>
                <div className="flex items-center gap-2">
                  <span className="font-medium">{wallet.label}</span>
                  <span className="text-muted-foreground font-mono text-xs">
                    {shortAddress(wallet.address)}
                  </span>
                  <Badge variant="secondary" className="text-xs">
                    {wallet.wallet_type === 'mnemonic' ? 'Mnemonic' : 'Private Key'}
                  </Badge>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {selectedWallet && (
        <div className="mt-3 pt-3 border-t border-border/50">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Connected Address:</span>
            <span className="font-mono text-foreground">{shortAddress(selectedWallet.address)}</span>
          </div>
        </div>
      )}
    </Card>
  );
};

