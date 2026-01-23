import React from 'react';
import { Card } from '@/components/ui/card';
import { AssetAllocation } from '../types';

interface AllocationCardProps {
    allocation: AssetAllocation[];
}

export const AllocationCard: React.FC<AllocationCardProps> = ({ allocation }) => {
    return (
        <Card className="p-6 glass-card text-left">
            <h3 className="text-sm font-semibold text-foreground mb-6 flex items-center gap-2">
                Allocation
            </h3>
            {allocation.length > 0 ? (
                <div className="space-y-4">
                    {allocation.map((asset, index) => (
                        <div key={index} className="space-y-2 group">
                            <div className="flex justify-between items-center text-sm">
                                <div className="flex items-center gap-2">
                                    <span className="font-medium text-foreground">{asset.symbol}</span>
                                    <span className="text-xs text-muted-foreground">{asset.name}</span>
                                </div>
                                <div className="text-right">
                                    <span className="font-mono text-foreground">{asset.percentage.toFixed(1)}%</span>
                                    <span className="text-xs text-muted-foreground ml-2 font-mono">
                                        ${asset.value_usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                                    </span>
                                </div>
                            </div>
                            <div className="w-full bg-muted/30 rounded-full h-1 overflow-hidden">
                                <div
                                    className={`h-full ${asset.color} opacity-80 rounded-full transition-all duration-500`}
                                    style={{ width: `${Math.max(asset.percentage, 1)}%` }}
                                />
                            </div>
                        </div>
                    ))}
                </div>
            ) : (
                <div className="text-center py-8 text-muted-foreground">
                    <p className="text-xs">No assets found</p>
                </div>
            )}
        </Card>
    );
};
