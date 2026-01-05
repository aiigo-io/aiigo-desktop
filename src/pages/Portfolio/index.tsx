import React from 'react';
import BitcoinAssets from './components/BitcoinAssets';
import EvmAssets from './components/EvmAssets';

const Portfolio: React.FC = () => {
  return (
    <div className="min-h-screen p-6 font-sans bg-background">
      {/* Background Effects */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute top-[-20%] right-[-10%] w-[50%] h-[50%] bg-primary/5 rounded-full blur-[150px]" />
        <div className="absolute bottom-[-20%] left-[-10%] w-[50%] h-[50%] bg-purple-500/5 rounded-full blur-[150px]" />
      </div>

      <div className="relative z-10 max-w-[1400px] mx-auto space-y-6">
        {/* Page Header */}
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight text-foreground">
            Portfolio
          </h1>
          <p className="text-muted-foreground text-sm">
            Manage your crypto assets across Bitcoin and EVM chains
          </p>
        </div>

        {/* Assets Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <BitcoinAssets />
          <EvmAssets />
        </div>
      </div>
    </div>
  );
};

export default Portfolio;