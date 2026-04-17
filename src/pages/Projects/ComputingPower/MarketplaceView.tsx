
import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Activity, Users, TrendingUp, ShieldCheck, Zap, Search, Server, Cloud, Box } from 'lucide-react';
import { useComputingStore } from './store/useComputingStore';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';

const MarketplaceView: React.FC = () => {
    const { setActiveTab, nodes, tasks } = useComputingStore();

    const activeNodesParams = Object.values(nodes).filter(n => n.status === 'Active').length;
    const totalPower = Object.values(nodes).reduce((acc, n) => acc + n.computePower, 0);
    const activeTasksCount = Object.values(tasks).filter(t => t.status === 'InProgress').length;

    const stats = [
        { label: 'Network Hashrate', value: `${(totalPower / 1000).toFixed(2)} EH/s`, icon: Activity, change: '+12.5%' },
        { label: 'Active Providers', value: activeNodesParams.toLocaleString(), icon: Users, change: '+8.2%' },
        { label: 'Active Tasks', value: activeTasksCount.toLocaleString(), icon: TrendingUp, change: '+15.4%' },
        { label: 'Platform Fee', value: '8%', icon: ShieldCheck, change: 'Stable' },
    ];

    const tiers = [
        {
            name: 'Tier 1: Enterprise',
            description: 'AIIGO-owned premium infrastructure with 99.99% SLA.',
            icon: Server,
            color: 'text-blue-500',
            bg: 'bg-blue-500/10',
            stats: '3,240 Nodes'
        },
        {
            name: 'Tier 2: Cloud Partner',
            description: 'Hyper-scaled capacity through AWS/GCP/Azure partnerships.',
            icon: Cloud,
            color: 'text-purple-500',
            bg: 'bg-purple-500/10',
            stats: '45,000 Nodes'
        },
        {
            name: 'Tier 3: Community',
            description: 'Decentralized global marketplace of individual providers.',
            icon: Box,
            color: 'text-emerald-500',
            bg: 'bg-emerald-500/10',
            stats: '106,580 Nodes'
        }
    ];

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {/* Header Section */}
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
                        Decentralized Compute <br />
                        <span className="bg-clip-text text-transparent bg-gradient-to-r from-blue-400 via-primary to-purple-500 drop-shadow-sm">
                            Powered by AIIGO
                        </span>
                    </h1>
                    <p className="text-muted-foreground text-lg md:text-xl max-w-2xl leading-relaxed font-medium">
                        Connect idle resource providers with AI compute buyers. Earn passive income or access affordable distributed power through our optimized matching engine.
                    </p>
                    <div className="flex flex-wrap gap-5 pt-4">
                        <Button size="lg" onClick={() => setActiveTab('buyer')} className="h-[60px] px-10 bg-primary hover:bg-primary/90 text-primary-foreground shadow-[0_10px_40px_-10px_rgba(var(--primary),0.3)] rounded-2xl font-black text-base gap-3 transition-all hover:scale-105 active:scale-95">
                            <Search className="w-5 h-5 stroke-[3]" />
                            Rent Power
                        </Button>
                        <Button size="lg" variant="ghost" onClick={() => setActiveTab('provider')} className="h-[60px] px-10 border border-white/10 bg-white/[0.03] hover:bg-white/[0.08] backdrop-blur-xl rounded-2xl font-black text-base gap-3 transition-all hover:scale-105 hover:border-primary/30">
                            <Zap className="w-5 h-5 text-primary" />
                            Become Provider
                        </Button>
                    </div>
                </div>
            </div>

            {/* Marketplace Metrics */}
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
                {stats.map((stat, i) => (
                    <Card key={i} className="bg-white/[0.03] backdrop-blur-xl border-white/[0.05] hover:border-primary/30 hover:bg-white/[0.05] transition-all duration-300 group hover:-translate-y-1 shadow-xl">
                        <CardContent className="p-8">
                            <div className="flex justify-between items-start mb-6">
                                <div className="p-4 rounded-2xl bg-primary/5 text-primary group-hover:scale-110 group-hover:bg-primary/10 transition-all duration-300">
                                    <stat.icon className="w-6 h-6 stroke-[2.5]" />
                                </div>
                                <div className={cn("text-[11px] font-black px-2.5 py-1 rounded-full border",
                                    stat.change.startsWith('+') ? "border-emerald-500/20 bg-emerald-500/10 text-emerald-500" : "border-primary/20 bg-primary/10 text-primary"
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

            {/* Infrastructure Tiers */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-8 pt-4">
                {tiers.map((tier, i) => (
                    <Card key={i} className="bg-white/[0.02] backdrop-blur-md border border-white/[0.05] group hover:bg-white/[0.05] transition-all duration-500 flex flex-col relative overflow-hidden">
                        <div className={cn("absolute top-0 right-0 w-24 h-24 blur-[60px] opacity-20 transition-all group-hover:opacity-40", tier.bg)} />
                        <CardHeader className="relative pt-8 pb-6">
                            <div className={cn("w-14 h-14 rounded-2xl flex items-center justify-center mb-6 shadow-xl transition-all group-hover:scale-110 group-hover:rotate-3", tier.bg)}>
                                <tier.icon className={cn("w-7 h-7", tier.color)} />
                            </div>
                            <CardTitle className="text-2xl font-black">{tier.name}</CardTitle>
                            <CardDescription className="text-sm font-medium leading-relaxed min-h-[48px] text-muted-foreground/80">{tier.description}</CardDescription>
                        </CardHeader>
                        <CardContent className="mt-auto px-6 py-8 border-t border-white/[0.05] flex items-center justify-between relative bg-white/[0.01]">
                            <span className="text-sm font-mono font-black text-foreground/70">{tier.stats}</span>
                            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-emerald-500/5 border border-emerald-500/10">
                                <span className="relative flex h-2 w-2">
                                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                                    <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                                </span>
                                <span className="text-[10px] font-black text-emerald-500 uppercase tracking-tighter">Available</span>
                            </div>
                        </CardContent>
                    </Card>
                ))}
            </div>

            {/* All Nodes List */}
            <Card className="bg-white/[0.02] border-white/[0.05] py-6">
                <CardHeader>
                    <CardTitle>Verified Global Nodes</CardTitle>
                    <CardDescription>Real-time availability of decentralized computing resources.</CardDescription>
                </CardHeader>
                <CardContent>
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
                                {Object.values(nodes).map((node) => (
                                    <tr key={node.nodeId} className="group hover:bg-white/[0.03] transition-all">
                                        <td className="px-6 py-4 font-mono text-sm text-foreground/80">{node.nodeId}</td>
                                        <td className="px-6 py-4">
                                            <Badge variant="secondary" className="text-[10px]">{node.resourceType}</Badge>
                                        </td>
                                        <td className="px-6 py-4 text-sm font-bold">{node.metadata.gpuModel}</td>
                                        <td className="px-6 py-4 text-sm text-muted-foreground">{node.metadata.region}</td>
                                        <td className="px-6 py-4 text-sm font-mono text-emerald-400">{node.computePower > 0 ? `${node.computePower} TFLOPS` : '-'}</td>
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
                </CardContent>
            </Card>
        </div>
    );
};

export default MarketplaceView;
