
import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Activity, Search, Server, ShieldCheck, TrendingUp, Users, Zap } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import type { ComputeMarketplaceModel } from './useComputeMarketplace';

const MarketplaceView: React.FC<{ model: ComputeMarketplaceModel }> = ({ model }) => {
    const { setActiveTab, snapshot, config } = model;

    const activeNodeCount = snapshot.activeNodes.length;
    const totalPower = snapshot.activeNodes.reduce((acc, node) => acc + node.computePower, 0);
    const activeTaskCount = snapshot.providerTasks.filter((task) => task.status === 'Assigned' || task.status === 'InProgress').length;

    const stats = [
        { label: 'Active Compute Power', value: `${totalPower.toLocaleString()} units`, icon: Activity, change: 'Chain-tracked' },
        { label: 'Active Providers', value: activeNodeCount.toLocaleString(), icon: Users, change: config.chainName },
        { label: 'Live Assigned Tasks', value: activeTaskCount.toLocaleString(), icon: TrendingUp, change: 'Escrow-backed' },
        { label: 'Platform Fee', value: '8%', icon: ShieldCheck, change: 'Contract-enforced' },
    ];

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-12 pt-4">
                <div className="space-y-6 flex-1">
                    <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-primary/5 border border-primary/20 text-[11px] font-bold text-primary uppercase tracking-wider backdrop-blur-sm">
                        <span className="relative flex h-2 w-2">
                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
                            <span className="relative inline-flex rounded-full h-2 w-2 bg-primary"></span>
                        </span>
                        Computing Power Marketplace
                    </div>
                    <h1 className="text-5xl md:text-6xl font-black tracking-tight text-foreground leading-[1.05]">
                        Contract-Anchored Compute <br />
                        <span className="bg-clip-text text-transparent bg-gradient-to-r from-blue-400 via-primary to-purple-500 drop-shadow-sm">
                            Powered by AIIGO
                        </span>
                    </h1>
                    <p className="text-muted-foreground text-lg md:text-xl max-w-2xl leading-relaxed font-medium">
                        This surface no longer invents node or task state locally. Providers, funded tasks, escrow, and disputes only appear after an explicit chain refresh against configured contracts.
                    </p>
                    <div className="flex flex-wrap gap-5 pt-4">
                        <Button size="lg" onClick={() => setActiveTab('buyer')} className="h-[60px] px-10 bg-primary hover:bg-primary/90 text-primary-foreground shadow-[0_10px_40px_-10px_rgba(var(--primary),0.3)] rounded-2xl font-black text-base gap-3 transition-all hover:scale-105 active:scale-95">
                            <Search className="w-5 h-5 stroke-[3]" />
                            Buyer Flow
                        </Button>
                        <Button size="lg" variant="ghost" onClick={() => setActiveTab('provider')} className="h-[60px] px-10 border border-white/10 bg-white/[0.03] hover:bg-white/[0.08] backdrop-blur-xl rounded-2xl font-black text-base gap-3 transition-all hover:scale-105 hover:border-primary/30">
                            <Zap className="w-5 h-5 text-primary" />
                            Provider Flow
                        </Button>
                    </div>
                </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
                {stats.map((stat, i) => (
                    <Card key={i} className="bg-white/[0.03] backdrop-blur-xl border-white/[0.05] hover:border-primary/30 hover:bg-white/[0.05] transition-all duration-300 group hover:-translate-y-1 shadow-xl">
                        <CardContent className="p-8">
                            <div className="flex justify-between items-start mb-6">
                                <div className="p-4 rounded-2xl bg-primary/5 text-primary group-hover:scale-110 group-hover:bg-primary/10 transition-all duration-300">
                                    <stat.icon className="w-6 h-6 stroke-[2.5]" />
                                </div>
                                <div className={cn("text-[11px] font-black px-2.5 py-1 rounded-full border",
                                    stat.change === 'Escrow-backed' ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-500' : 'border-primary/20 bg-primary/10 text-primary'
                                )}>
                                    {stat.change}
                                </div>
                            </div>
                            <p className="text-xs font-bold text-muted-foreground uppercase tracking-widest font-mono">{stat.label}</p>
                            <p className="text-3xl font-black text-foreground mt-2 tracking-tight">{stat.value}</p>
                        </CardContent>
                    </Card>
                ))}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-8 pt-4">
                {[
                    {
                        name: 'Explicit Query Path',
                        description: 'Wallet loading is local. Chain state enters only through the refresh path so UI reads do not smuggle in hidden network mutation.',
                    },
                    {
                        name: 'Escrow Before Work',
                        description: 'Open tasks shown here are contract-funded tasks only. Unfunded work no longer appears as executable opportunity.',
                    },
                    {
                        name: 'One Settlement Truth',
                        description: 'Approval, dispute, and payout status come from contract state after refresh instead of Zustand or local optimistic flags.',
                    },
                ].map((item) => (
                    <Card key={item.name} className="bg-white/[0.02] backdrop-blur-md border border-white/[0.05]">
                        <CardHeader className="pt-8 pb-6">
                            <div className="w-14 h-14 rounded-2xl flex items-center justify-center mb-6 shadow-xl bg-primary/10 text-primary">
                                <Server className="w-7 h-7" />
                            </div>
                            <CardTitle className="text-2xl font-black">{item.name}</CardTitle>
                            <CardDescription className="text-sm font-medium leading-relaxed text-muted-foreground/80">{item.description}</CardDescription>
                        </CardHeader>
                    </Card>
                ))}
            </div>

            <Card className="bg-white/[0.02] border-white/[0.05] py-6">
                <CardHeader>
                    <CardTitle>Verified Global Nodes</CardTitle>
                    <CardDescription>Chain-refreshed active providers pulled from NodeRegistry.</CardDescription>
                </CardHeader>
                <CardContent>
                    {snapshot.activeNodes.length === 0 ? (
                        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-8 text-sm text-muted-foreground">
                            No active nodes loaded yet. Configure contracts and use the refresh action to query the live provider set.
                        </div>
                    ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full">
                            <thead>
                                <tr className="border-b border-white/[0.05] text-[10px] font-black text-muted-foreground/60 uppercase tracking-[0.2em] text-left">
                                    <th className="px-6 py-4">Node ID</th>
                                    <th className="px-6 py-4">Type</th>
                                    <th className="px-6 py-4">Model</th>
                                    <th className="px-6 py-4">Region</th>
                                    <th className="px-6 py-4">Power</th>
                                    <th className="px-6 py-4">Reputation</th>
                                    <th className="px-6 py-4 text-right">Status</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-white/[0.05]">
                                {snapshot.activeNodes.map((node) => (
                                    <tr key={node.nodeId} className="group hover:bg-white/[0.03] transition-all">
                                        <td className="px-6 py-4 font-mono text-sm text-foreground/80">{node.nodeId}</td>
                                        <td className="px-6 py-4">
                                            <Badge variant="secondary" className="text-[10px]">{node.resourceType}</Badge>
                                        </td>
                                        <td className="px-6 py-4 text-sm font-bold">{node.metadata.gpuModel}</td>
                                        <td className="px-6 py-4 text-sm text-muted-foreground">{node.metadata.region}</td>
                                        <td className="px-6 py-4 text-sm font-mono text-emerald-400">{node.computePower > 0 ? `${node.computePower}` : '-'}</td>
                                        <td className="px-6 py-4 text-sm">{node.reputation}</td>
                                        <td className="px-6 py-4 text-right">
                                            <div className={cn("inline-flex items-center px-2 py-0.5 rounded text-[10px] font-black uppercase tracking-wider",
                                                node.status === 'Active' ? "bg-emerald-500/10 text-emerald-500" :
                                                    node.status === 'Pending' ? "bg-yellow-500/10 text-yellow-500" :
                                                        "bg-white/5 text-muted-foreground"
                                            )}>
                                                {node.status}
                                            </div>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                    )}
                </CardContent>
            </Card>

            <Card className="bg-white/[0.02] border-white/[0.05] py-6">
                <CardHeader>
                    <CardTitle>Funded Open Tasks</CardTitle>
                    <CardDescription>Only chain-funded tasks are eligible for provider acceptance.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                    {snapshot.openTasks.length === 0 ? (
                        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-6 text-sm text-muted-foreground">
                            No funded tasks are available in the current snapshot.
                        </div>
                    ) : snapshot.openTasks.slice(0, 5).map((task) => (
                        <div key={task.taskId} className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
                            <div className="flex items-center justify-between gap-4">
                                <div>
                                    <div className="flex items-center gap-2">
                                        <span className="text-lg font-bold">{task.specTitle}</span>
                                        <Badge variant="outline" className="text-[10px]">{task.resourceType}</Badge>
                                    </div>
                                    <p className="mt-2 text-sm text-muted-foreground">{task.requiredPower} power · {Math.round(task.durationSeconds / 3600)}h · trust {task.minTrustLevel}+</p>
                                </div>
                                <div className="text-right">
                                    <div className="font-mono text-lg font-bold text-emerald-400">{task.escrowAmountEth.toFixed(4)} ETH</div>
                                    <div className="text-xs text-muted-foreground">Escrow funded</div>
                                </div>
                            </div>
                        </div>
                    ))}
                </CardContent>
            </Card>
        </div>
    );
};

export default MarketplaceView;
