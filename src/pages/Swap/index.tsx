import React, { useState } from 'react';
import { SwapCard } from './components/SwapCard';
import { WalletSelector } from './components/WalletSelector';

interface WalletInfo {
  id: string;
  label: string;
  wallet_type: 'mnemonic' | 'private-key';
  address: string;
  balance: number;
  created_at: string;
  updated_at: string;
}

const SWAP: React.FC = () => {
  const [selectedWallet, setSelectedWallet] = useState<WalletInfo | null>(null);

  return (
    <div className="min-h-screen p-6 font-sans">
      {/* Background Effects */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute top-[-20%] left-[-10%] w-[50%] h-[50%] bg-primary/5 rounded-full blur-[150px]" />
        <div className="absolute bottom-[-20%] right-[-10%] w-[50%] h-[50%] bg-purple-500/5 rounded-full blur-[150px]" />
      </div>

      <div className="relative z-10 max-w-7xl mx-auto space-y-6">
        {/* Header */}
        <div className="text-center md:text-left">
          <h1 className="text-3xl font-bold tracking-tight text-foreground">
            Token Swap
          </h1>
          <p className="text-muted-foreground mt-1 text-sm">
            Swap tokens across multiple chains with the best rates
          </p>
        </div>

        {/* Wallet Selector */}
        <div className="max-w-[480px] mx-auto">
          <WalletSelector onWalletChange={setSelectedWallet} />
        </div>

        {/* Swap Form */}
        <div className="flex justify-center">
          <SwapCard wallet={selectedWallet} />
        </div>

        {/* Wallet Info Display */}
        {selectedWallet && (
          <div className="max-w-[480px] mx-auto text-center text-xs text-muted-foreground/50 font-mono">
            Active: {selectedWallet.label}
          </div>
        )}
      </div>
    </div>
  );
}

export default SWAP;