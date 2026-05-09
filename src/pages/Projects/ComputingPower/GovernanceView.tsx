import React from 'react';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Gavel, Scale, Settings } from 'lucide-react';

import type { ComputeMarketplaceModel } from './useComputeMarketplace';

const GovernanceView: React.FC<{ model: ComputeMarketplaceModel }> = ({ model }) => {
    const { config, snapshot } = model;
    const disputes = snapshot.disputes;

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="flex justify-between items-center">
                <div>
                    <h2 className="text-3xl font-black tracking-tight">Governance & Admin</h2>
                    <p className="text-muted-foreground">This page now reports live dispute and escrow posture only. Demo reset controls were removed.</p>
                </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">System Status</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-emerald-400">Configured</div>
                        <p className="text-xs text-muted-foreground">{config.isConfigured ? config.chainName : 'Missing contract addresses'}</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">Active Disputes</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-yellow-400">{disputes.length}</div>
                        <p className="text-xs text-muted-foreground">Pending owner resolution</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">Total Escrow Locked</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-primary">{snapshot.totalLockedEscrowEth.toFixed(4)} ETH</div>
                        <p className="text-xs text-muted-foreground">Open settlement obligations</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">Buyer Refund Queue</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-blue-400">{snapshot.pendingBuyerRefundEth.toFixed(4)} ETH</div>
                        <p className="text-xs text-muted-foreground">Claimable after refund settlement</p>
                    </CardContent>
                </Card>
            </div>

            <Card className="bg-white/[0.02] border-white/[0.05]">
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Gavel className="w-5 h-5" /> Arbitration Queue
                    </CardTitle>
                </CardHeader>
                <CardContent>
                    {disputes.length === 0 ? (
                        <div className="text-center py-8 text-muted-foreground">
                            No active disputes in the current chain snapshot.
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {disputes.map((task) => (
                                <div key={task.taskId} className="rounded-lg border border-white/5 bg-white/[0.03] p-4">
                                    <div className="flex items-start justify-between gap-6">
                                        <div>
                                            <div className="font-bold text-lg">{task.specTitle}</div>
                                            <div className="text-sm text-muted-foreground">Task ID: {task.taskId}</div>
                                            <div className="text-sm text-muted-foreground mt-2">Reason: {task.disputeReason ?? 'Not exposed in snapshot'}</div>
                                        </div>
                                        <div className="text-right text-sm text-muted-foreground">
                                            <div>Resolved: {task.resolved ? 'Yes' : 'No'}</div>
                                            <div>Gross provider amount: {task.grossProviderAmountEth.toFixed(4)} ETH</div>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </CardContent>
            </Card>

            <Card className="bg-white/[0.02] border-white/[0.05]">
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Settings className="w-5 h-5" /> Protocol Parameters
                    </CardTitle>
                </CardHeader>
                <CardContent>
                    <div className="grid grid-cols-2 lg:grid-cols-4 gap-6">
                        {[
                            { label: 'Platform Fee', val: '8.00%' },
                            { label: 'Minimum Stake', val: '0.5 ETH' },
                            { label: 'Settlement Model', val: 'Claimable balances' },
                            { label: 'Dispute Resolver', val: 'Contract owner / multisig' },
                        ].map((parameter) => (
                            <div key={parameter.label} className="space-y-1">
                                <span className="text-xs uppercase font-bold text-muted-foreground">{parameter.label}</span>
                                <div className="font-mono text-xl font-bold bg-white/5 px-3 py-2 rounded">{parameter.val}</div>
                            </div>
                        ))}
                    </div>
                </CardContent>
            </Card>

            <Card className="bg-white/[0.02] border-white/[0.05]">
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Scale className="w-5 h-5" /> Scope Boundary
                    </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm text-muted-foreground">
                    <p>Only ETH testnet flows are assumed here.</p>
                    <p>Refunds and payouts are modeled as claimable balances, not immediate wallet assumptions.</p>
                    <p>This UI does not resolve disputes locally and does not emulate arbitration outcomes.</p>
                </CardContent>
            </Card>
        </div>
    );
};

export default GovernanceView;
