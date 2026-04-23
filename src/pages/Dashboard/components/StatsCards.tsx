import React from 'react';
import { Card } from '@/components/ui/card';
import { Wallet, Activity, ArrowUpRight, ArrowDownLeft } from 'lucide-react';
import { DashboardStats } from '../types';

interface StatsCardsProps {
    stats: DashboardStats;
}

export const StatsCards: React.FC<StatsCardsProps> = ({ stats }) => {
    const freshnessLabel = stats.freshness.status;
    const updatedAt = stats.freshness.updated_at
        ? new Date(stats.freshness.updated_at * 1000).toLocaleTimeString()
        : null;
    const failedSources = stats.freshness.failed_sources.join(', ');

    return (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Total Balance Card */}
            <Card className="p-6 glass-card relative overflow-hidden group">
                <div className="relative z-10">
                    <div className="flex items-center justify-between mb-2">
                        <h3 className="text-[11px] font-bold text-muted-foreground uppercase tracking-widest font-mono">Total Balance</h3>
                        <Wallet className="w-4 h-4 text-muted-foreground/50" />
                    </div>
                    <div className="mb-3 flex items-center gap-2 text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
                        <span className="rounded-sm bg-muted px-1.5 py-0.5">{freshnessLabel}</span>
                        {updatedAt && <span>{updatedAt}</span>}
                    </div>
                    {failedSources && (
                        <p className="mb-3 text-[11px] font-mono text-amber-500">
                            Partial sources: {failedSources}
                        </p>
                    )}
                    <div className="space-y-1">
                        <p className="text-4xl font-light tracking-tight text-foreground font-mono">
                            {stats.total_balance_usd}
                        </p>
                        <p className="text-xs font-mono text-muted-foreground/80 flex items-center gap-2">
                            <span className="text-primary">{stats.total_balance_btc}</span>
                            <span className="text-[10px] px-1.5 py-0.5 rounded-sm bg-muted text-muted-foreground">BTC</span>
                        </p>
                    </div>
                </div>
            </Card>

            {/* 24h Change Card */}
            <Card className="p-6 glass-card relative overflow-hidden group">
                <div className="relative z-10">
                    <div className="flex items-center justify-between mb-2">
                        <h3 className="text-[11px] font-bold text-muted-foreground uppercase tracking-widest font-mono">24h Performance</h3>
                        <Activity className="w-4 h-4 text-muted-foreground/50" />
                    </div>
                    <div className="flex items-end justify-between">
                        <div>
                            <p className={`text-4xl font-light tracking-tight font-mono ${stats.change_24h_amount.startsWith('+') ? 'text-emerald-400' : 'text-destructive'}`}>
                                {stats.change_24h_amount}
                            </p>
                            <p className="text-xs font-mono text-muted-foreground/80 mt-1">{stats.change_24h_percentage}</p>
                        </div>
                        {stats.change_24h_amount.startsWith('+') ? (
                            <ArrowUpRight className="w-8 h-8 text-emerald-500/20" />
                        ) : (
                            <ArrowDownLeft className="w-8 h-8 text-destructive/20" />
                        )}
                    </div>
                </div>
            </Card>
        </div>
    );
};
