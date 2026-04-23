import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/card';
import { ArrowUpRight, ArrowDownLeft, ShieldCheck, Code } from 'lucide-react';
import { shortAddress } from '@/lib/utils';
import { UnifiedTransaction } from '../types';

interface RecentTransactionsProps {
    transactions: UnifiedTransaction[];
}

export const RecentTransactions: React.FC<RecentTransactionsProps> = ({ transactions }) => {
    const navigate = useNavigate();

    const getStatusClass = (status: UnifiedTransaction['status']) => {
        switch (status) {
            case 'broadcasted':
                return 'bg-sky-500/10 text-sky-500 border-sky-500/20';
            case 'pending':
                return 'bg-yellow-500/10 text-yellow-500 border-yellow-500/20';
            case 'confirmed':
                return 'bg-green-500/10 text-green-500 border-green-500/20';
            case 'failed':
                return 'bg-red-500/10 text-red-500 border-red-500/20';
            case 'replaced':
                return 'bg-orange-500/10 text-orange-500 border-orange-500/20';
            case 'dropped':
                return 'bg-slate-500/10 text-slate-500 border-slate-500/20';
        }
    };

    const formatStatusLabel = (status: UnifiedTransaction['status']) => {
        switch (status) {
            case 'broadcasted':
                return 'Broadcasted';
            case 'pending':
                return 'Pending';
            case 'confirmed':
                return 'Confirmed';
            case 'failed':
                return 'Failed';
            case 'replaced':
                return 'Replaced';
            case 'dropped':
                return 'Dropped';
        }
    };

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

    const getTransactionPresentation = (tx: UnifiedTransaction) => {
        switch (tx.tx_type) {
            case 'send':
                return {
                    label: 'Send',
                    icon: <ArrowUpRight className="w-3 h-3" />,
                    textClass: 'text-destructive',
                    amountPrefix: '-',
                };
            case 'receive':
                return {
                    label: 'Receive',
                    icon: <ArrowDownLeft className="w-3 h-3" />,
                    textClass: 'text-emerald-400',
                    amountPrefix: '+',
                };
            case 'approve':
                return {
                    label: 'Approve',
                    icon: <ShieldCheck className="w-3 h-3" />,
                    textClass: 'text-blue-500',
                    amountPrefix: '-',
                };
            case 'contract':
                return {
                    label: 'Contract',
                    icon: <Code className="w-3 h-3" />,
                    textClass: 'text-purple-500',
                    amountPrefix: '-',
                };
        }
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
                                    <th className="px-4 py-2">Status</th>
                                    <th className="px-4 py-2">Hash</th>
                                    <th className="px-4 py-2 text-right">Amount</th>
                                    <th className="px-4 py-2 rounded-r-sm text-right">Time</th>
                                </tr>
                            </thead>
                            <tbody>
                                {transactions.map((tx) => {
                                    const presentation = getTransactionPresentation(tx);
                                    return (
                                        <tr key={tx.id} className="border-b border-border/50 hover:bg-muted/10 transition-colors">
                                            <td className="px-4 py-3">
                                                <span className={`inline-flex items-center gap-1.5 font-medium ${presentation.textClass}`}>
                                                    {presentation.icon}
                                                    {presentation.label}
                                                </span>
                                            </td>
                                            <td className="px-4 py-3 font-medium text-foreground">
                                                {tx.asset_symbol}
                                            </td>
                                            <td className="px-4 py-3">
                                                <span className={`inline-flex rounded-full border px-2 py-0.5 text-[10px] font-mono uppercase ${getStatusClass(tx.status)}`}>
                                                    {formatStatusLabel(tx.status)}
                                                </span>
                                            </td>
                                            <td className="px-4 py-3 font-mono text-muted-foreground">
                                                {shortAddress(tx.tx_hash)}
                                            </td>
                                            <td className={`px-4 py-3 text-right font-mono ${presentation.textClass}`}>
                                                {presentation.amountPrefix}{tx.amount}
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
