import React from 'react';
import BitcoinAssets from './components/BitcoinAssets';
import EvmAssets from './components/EvmAssets';

const Portfolio: React.FC = () => {
  return (
    <div className="p-6 space-y-6">
      <div className="grid grid-cols-2 gap-6">
        <BitcoinAssets />
        <EvmAssets />
      </div>  
    </div>
  )
}

export default Portfolio;