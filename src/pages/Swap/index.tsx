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
    <div className="min-h-screen bg-slate-50 p-6">
      {/* Background Effects */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-blue-200/20 rounded-full blur-[120px]" />
        <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-purple-200/20 rounded-full blur-[120px]" />
      </div>

      <div className="relative z-10 max-w-7xl mx-auto space-y-6">
        {/* Header */}
        <div>
          <h1 className="text-4xl font-bold bg-gradient-to-r from-slate-900 to-slate-600 bg-clip-text text-transparent">
            Token Swap
          </h1>
          <p className="text-slate-500 mt-1">
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
          <div className="max-w-[480px] mx-auto text-center text-xs text-slate-400">
            Using wallet: {selectedWallet.label}
          </div>
        )}
      </div>
    </div>
  );
}

export default SWAP;