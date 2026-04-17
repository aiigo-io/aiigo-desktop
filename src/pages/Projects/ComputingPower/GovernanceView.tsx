
import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { useComputingStore } from './store/useComputingStore'; // Assuming this exists or will exist
import { RefreshCw, Gavel, Scale, Settings } from 'lucide-react';

// Assuming you might add dispute related items to the store later, 
// for now we can mock some dispute data or just show the admin controls.

const GovernanceView: React.FC = () => {
    const { reset, tasks } = useComputingStore();

    // Calculate some system stats
    const disputes = Object.values(tasks).filter(t => t.status === 'Disputed');
    const totalEscrow = Object.values(tasks).reduce((acc, t) => acc + (t.status !== 'Verified' && t.status !== 'Cancelled' ? t.escrowAmount : 0), 0);

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="flex justify-between items-center">
                <div>
                    <h2 className="text-3xl font-black tracking-tight">Governance & Admin</h2>
                    <p className="text-muted-foreground">System parameter controls and dispute resolution (Multi-Sig emulation).</p>
                </div>
                <Button variant="destructive" onClick={reset}>
                    <RefreshCw className="w-4 h-4 mr-2" />
                    Reset Demo State
                </Button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">System Status</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-emerald-400">Operational</div>
                        <p className="text-xs text-muted-foreground">All contracts unpaused</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">Active Disputes</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-yellow-400">{disputes.length}</div>
                        <p className="text-xs text-muted-foreground">Pending Arbitration</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05]">
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground uppercase">Total Escrow Locked</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-primary">{totalEscrow.toFixed(4)} ETH</div>
                        <p className="text-xs text-muted-foreground">In active contracts</p>
                    </CardContent>
                </Card>
            </div>

            {/* Arbitration Queue */}
            <Card className="bg-white/[0.02] border-white/[0.05]">
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Gavel className="w-5 h-5" /> Arbitration Queue
                    </CardTitle>
                </CardHeader>
                <CardContent>
                    {disputes.length === 0 ? (
                        <div className="text-center py-8 text-muted-foreground">
                            No active disputes. The system is running smoothly.
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {disputes.map(task => (
                                <div key={task.taskId} className="flex items-center justify-between p-4 bg-white/[0.03] rounded-lg border border-white/5">
                                    <div>
                                        <div className="font-bold text-lg">{task.specTitle}</div>
                                        <div className="text-sm text-muted-foreground">Task ID: {task.taskId}</div>
                                    </div>
                                    <div className="flex gap-2">
                                        <Button size="sm" variant="outline">View Evidence</Button>
                                        <Button size="sm" className="bg-emerald-600 hover:bg-emerald-700">Rule for Provider</Button>
                                        <Button size="sm" variant="destructive">Rule for Buyer</Button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </CardContent>
            </Card>

            {/* Config Board */}
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
                            { label: 'Min Stake (Basic)', val: '0.5 ETH' },
                            { label: 'Arbitration Fee', val: '2.00%' },
                            { label: 'Slashed Burn Rate', val: '50%' },
                        ].map((p, i) => (
                            <div key={i} className="space-y-1">
                                <span className="text-xs uppercase font-bold text-muted-foreground">{p.label}</span>
                                <div className="font-mono text-xl font-bold bg-white/5 px-3 py-2 rounded">{p.val}</div>
                            </div>
                        ))}
                    </div>
                </CardContent>
            </Card>
        </div>
    );
};

export default GovernanceView;
