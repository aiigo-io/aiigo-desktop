import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/card';
import { ArrowUpRight, ArrowDownLeft } from 'lucide-react';
import { shortAddress } from '@/lib/utils';
import { UnifiedTransaction } from '../types';

interface RecentTransactionsProps {
    transactions: UnifiedTransaction[];
}

export const RecentTransactions: React.FC<RecentTransactionsProps> = ({ transactions }) => {
    const navigate = useNavigate();

    const formatTimeAgo = (timestamp: string) => {
        const now = new Date();
        const txDate = new Date(timestamp);
        const diffMs = now.getTime() - txDate.getTime();
        const diffMins = Math.floor(diffMs / 60000);
        const diffHours = Math.floor(diffMs / 3600000);
        const diffDays = Math.floor(diffMs / 86400000);

        if (diffMins < 1) return 'Just now';
        if (diffMins < 60) return `${diffMins} minute${diffMins > 1 ? 's' : ''} ago`;
        if (diffHours < 24) return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
        if (diffDays < 7) return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
        return txDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    };

    return (
        <Card className="p-6 glass-card text-left">
            <div className="flex items-center justify-between mb-6">
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                    Recent Activity
                </h3>
                <button
                    onClick={() => navigate('/transactions')}
                    className="text-xs font-medium text-muted-foreground hover:text-foreground transition-colors"
                >
                    View All
                </button>
            </div>

            <div className="space-y-1">
                {transactions.length > 0 ? (
                    <div className="relative overflow-x-auto">
                        <table className="w-full text-xs text-left">
                            <thead className="text-[10px] text-muted-foreground uppercase bg-muted/20 font-mono">
                                <tr>
                                    <th className="px-4 py-2 rounded-l-sm">Type</th>
                                    <th className="px-4 py-2">Asset</th>
                                    <th className="px-4 py-2">Hash</th>
                                    <th className="px-4 py-2 text-right">Amount</th>
                                    <th className="px-4 py-2 rounded-r-sm text-right">Time</th>
                                </tr>
                            </thead>
                            <tbody>
                                {transactions.map((tx) => {
                                    const isSend = tx.tx_type === 'send';
                                    return (
                                        <tr key={tx.id} className="border-b border-border/50 hover:bg-muted/10 transition-colors">
                                            <td className="px-4 py-3">
                                                <span className={`inline-flex items-center gap-1.5 font-medium ${isSend ? 'text-destructive' : 'text-emerald-400'}`}>
                                                    {isSend ? (
                                                        <ArrowUpRight className="w-3 h-3" />
                                                    ) : (
                                                        <ArrowDownLeft className="w-3 h-3" />
                                                    )}
                                                    {isSend ? 'Send' : 'Receive'}
                                                </span>
                                            </td>
                                            <td className="px-4 py-3 font-medium text-foreground">
                                                {tx.asset_symbol}
                                            </td>
                                            <td className="px-4 py-3 font-mono text-muted-foreground">
                                                {shortAddress(tx.tx_hash)}
                                            </td>
                                            <td className={`px-4 py-3 text-right font-mono ${isSend ? 'text-destructive' : 'text-emerald-400'}`}>
                                                {isSend ? '-' : '+'}{tx.amount}
                                            </td>
                                            <td className="px-4 py-3 text-right text-muted-foreground">
                                                {formatTimeAgo(tx.timestamp)}
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                ) : (
                    <div className="text-center py-8 text-muted-foreground">
                        <p className="text-xs">No recent transactions</p>
                    </div>
                )}
            </div>
        </Card>
    );
};
