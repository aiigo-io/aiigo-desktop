import React from 'react';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
    Cpu,
    Zap,
    Wifi,
    Activity,
    TrendingUp,
    ShieldCheck,
    Box,
    Server,
    Cloud,
    Users,
    ArrowRight,
    Search,
    Lock,
    Smartphone,
    Globe,
    ShoppingBag,
    Target
} from 'lucide-react';
import { Progress } from '@/components/ui/progress';

const ComputingPower: React.FC = () => {
    const marketplaceStats = [
        { label: 'Network Hashrate', value: '1.24 EH/s', icon: Activity, change: '+12.5%' },
        { label: 'Active Providers', value: '154,820', icon: Users, change: '+8.2%' },
        { label: '24h Trading Vol', value: '$840,200', icon: TrendingUp, change: '+15.4%' },
        { label: 'Platform Fee (8%)', value: '$67,216', icon: ShieldCheck, change: 'Stable' },
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

    const popularResources = [
        { name: 'NVIDIA RTX 4090', type: 'GPU', rate: '$0.80/hr', util: 62, icon: Zap, color: 'text-orange-500' },
        { name: 'AMD EPYC 7763', type: 'CPU', rate: '$0.15/hr', util: 85, icon: Cpu, color: 'text-blue-400' },
        { name: '1Gbps Bandwidth', type: 'Network', rate: '$0.02/GB', util: 45, icon: Wifi, color: 'text-emerald-400' },
        { name: 'Edge Node 820', type: 'Mobile', rate: '$0.05/task', util: 78, icon: Smartphone, color: 'text-purple-400' },
        { name: 'IoT Data Point', type: 'Edge', rate: '$0.01/1k pts', util: 92, icon: Globe, color: 'text-cyan-400' },
    ];

    const participantRoles = {
        providers: [
            { type: 'GPU Provider', sub: 'AI Training, Rendering', model: 'Per GPU-hour', icon: Zap },
            { type: 'CPU Provider', sub: 'Data Processing, Compile', model: 'Per CPU-hour', icon: Cpu },
            { type: 'Network Provider', sub: 'Data Transfer, CDN', model: 'Per GB', icon: Wifi },
            { type: 'Mobile Provider', sub: 'Edge Computing, Testing', model: 'Per task', icon: Smartphone },
            { type: 'IoT/Edge Provider', sub: 'Sensor Data, Location', model: 'Per data point', icon: Activity },
        ],
        buyers: [
            { type: 'AI Startups', need: 'Model training, Inference', pay: 'Pay-as-you-go', icon: Target },
            { type: 'Researchers', need: 'Scientific simulations', pay: 'Grant-funded/Hourly', icon: Server },
            { type: 'Enterprises', need: 'Batch processing', pay: 'Volume contracts', icon: ShoppingBag },
            { type: 'Developers', need: 'CI/CD, Environments', pay: 'Per-minute billing', icon: Box },
        ]
    };

    return (
        <div className="min-h-screen p-6 font-sans overflow-x-hidden">
            {/* Ambient Background */}
            <div className="fixed inset-0 pointer-events-none overflow-hidden">
                <div className="absolute top-[-10%] right-[-10%] w-[60%] h-[60%] bg-blue-600/5 rounded-full blur-[120px] animate-pulse" />
                <div className="absolute bottom-[-10%] left-[-10%] w-[60%] h-[60%] bg-purple-600/5 rounded-full blur-[120px] animate-pulse delay-700" />
            </div>

            <div className="relative z-10 max-w-7xl mx-auto space-y-12 py-8">
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
                            <Button size="lg" className="h-[60px] px-10 bg-primary hover:bg-primary/90 text-primary-foreground shadow-[0_10px_40px_-10px_rgba(var(--primary),0.3)] rounded-2xl font-black text-base gap-3 transition-all hover:scale-105 active:scale-95">
                                <Search className="w-5 h-5 stroke-[3]" />
                                Rent Power
                            </Button>
                            <Button size="lg" variant="ghost" className="h-[60px] px-10 border border-white/10 bg-white/[0.03] hover:bg-white/[0.08] backdrop-blur-xl rounded-2xl font-black text-base gap-3 transition-all hover:scale-105 hover:border-primary/30">
                                <Zap className="w-5 h-5 text-primary" />
                                Become Provider
                            </Button>
                        </div>
                    </div>

                    <Card className="lg:w-[420px] bg-white/[0.03] backdrop-blur-3xl border-white/[0.05] shadow-2xl relative overflow-hidden group">
                        <div className="absolute inset-0 bg-gradient-to-br from-primary/10 via-transparent to-purple-500/5 opacity-50" />
                        <CardHeader className="relative py-4">
                            <div className="flex items-center justify-between">
                                <CardTitle className="text-xl flex items-center gap-2.5 font-black">
                                    <div className="p-2 rounded-xl bg-primary/10">
                                        <Lock className="w-5 h-5 text-primary" />
                                    </div>
                                    Provider Staking
                                </CardTitle>
                                <div className="px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-500 text-[10px] font-black uppercase tracking-widest border border-emerald-500/20">
                                    Active
                                </div>
                            </div>
                            <CardDescription className="text-sm font-medium pt-1">Lock AIIGO to increase trust level</CardDescription>
                        </CardHeader>
                        <CardContent className="relative space-y-8 pb-4">
                            <div className="grid grid-cols-2 gap-4">
                                <div className="space-y-1">
                                    <p className="text-[10px] text-muted-foreground uppercase font-black tracking-widest">Current Level</p>
                                    <p className="text-xl font-black text-foreground">Level 2: Verified</p>
                                </div>
                                <div className="text-right space-y-1">
                                    <p className="text-[10px] text-muted-foreground uppercase font-black tracking-widest">Staking APY</p>
                                    <p className="text-xl font-black text-emerald-400">12.5% <span className="text-[10px] text-muted-foreground align-middle ml-1">PA</span></p>
                                </div>
                            </div>
                            <div className="space-y-3">
                                <div className="flex justify-between items-end">
                                    <span className="text-xs font-bold text-muted-foreground uppercase tracking-tight">Progress to Trusted</span>
                                    <span className="text-sm font-black text-primary">65%</span>
                                </div>
                                <div className="relative">
                                    <Progress value={65} className="h-2.5 bg-white/5" />
                                    <div className="absolute inset-0 shadow-[0_0_20px_-5px_var(--primary)] pointer-events-none opacity-20" />
                                </div>
                                <p className="text-[11px] text-muted-foreground/80 leading-relaxed font-medium bg-white/[0.02] p-3 rounded-lg border border-white/5">
                                    Complete <span className="text-foreground font-bold">35 more tasks</span> to unlock Level 3 staking rewards and priority matching.
                                </p>
                            </div>
                            <Button className="w-full h-14 bg-primary/10 hover:bg-primary/20 text-primary border border-primary/20 hover:border-primary/40 transition-all font-black text-sm rounded-xl tracking-wide">
                                Manage Staking Portal
                            </Button>
                        </CardContent>
                    </Card>
                </div>

                {/* Marketplace Metrics */}
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
                    {marketplaceStats.map((stat, i) => (
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
                <div className="space-y-8 pt-4">
                    <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-l-4 border-primary/50 pl-6 py-2">
                        <div>
                            <h2 className="text-3xl font-black tracking-tight uppercase">Tiered Infrastructure</h2>
                            <p className="text-base text-muted-foreground font-medium mt-1">Diversified supply for every performance requirement</p>
                        </div>
                        <Button variant="link" className="text-sm font-black gap-2 text-primary p-0 h-auto self-start md:self-auto">
                            Technical Specs <ArrowRight className="w-4 h-4" />
                        </Button>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
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
                </div>

                {/* Ecosystem Participants */}
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 pt-8">
                    <Card className="bg-white/[0.02] border-white/[0.05] overflow-hidden group shadow-xl">
                        <CardHeader className="bg-primary/5 border-b border-white/[0.05] pt-8 pb-6 px-8">
                            <div className="flex items-center gap-3">
                                <div className="p-3 bg-primary text-primary-foreground rounded-2xl shadow-lg ring-4 ring-primary/10">
                                    <Users className="w-6 h-6" />
                                </div>
                                <div>
                                    <CardTitle className="text-2xl font-black uppercase tracking-tight">Resource Providers</CardTitle>
                                    <CardDescription className="text-sm font-semibold text-primary/80">Contribute computing power to earn passive income</CardDescription>
                                </div>
                            </div>
                        </CardHeader>
                        <CardContent className="p-0">
                            <div className="divide-y divide-white/[0.05]">
                                {participantRoles.providers.map((p, i) => (
                                    <div key={i} className="flex items-center justify-between p-6 px-8 hover:bg-white/[0.03] transition-all group">
                                        <div className="flex items-center gap-5">
                                            <div className="p-3 rounded-xl bg-white/[0.03] text-muted-foreground group-hover:text-primary group-hover:bg-primary/5 transition-all">
                                                <p.icon className="w-5 h-5" />
                                            </div>
                                            <div>
                                                <p className="text-base font-black text-foreground/90">{p.type}</p>
                                                <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider">{p.sub}</p>
                                            </div>
                                        </div>
                                        <div className="text-right">
                                            <p className="text-[11px] font-bold text-primary uppercase tracking-widest mb-1">Yield Model</p>
                                            <p className="text-xs font-mono font-black text-foreground/70">{p.model}</p>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </CardContent>
                    </Card>

                    <Card className="bg-white/[0.02] border-white/[0.05] overflow-hidden group shadow-xl">
                        <CardHeader className="bg-purple-500/5 border-b border-white/[0.05] pt-8 pb-6 px-8">
                            <div className="flex items-center gap-3">
                                <div className="p-3 bg-purple-500 text-white rounded-2xl shadow-lg ring-4 ring-purple-500/10">
                                    <ShoppingBag className="w-6 h-6" />
                                </div>
                                <div>
                                    <CardTitle className="text-2xl font-black uppercase tracking-tight text-white/90">Resource Buyers</CardTitle>
                                    <CardDescription className="text-sm font-semibold text-purple-400">High-performance compute for next-gen apps</CardDescription>
                                </div>
                            </div>
                        </CardHeader>
                        <CardContent className="p-0">
                            <div className="divide-y divide-white/[0.05]">
                                {participantRoles.buyers.map((b, i) => (
                                    <div key={i} className="flex items-center justify-between p-6 px-8 hover:bg-white/[0.03] transition-all group">
                                        <div className="flex items-center gap-5">
                                            <div className="p-3 rounded-xl bg-white/[0.03] text-muted-foreground group-hover:text-purple-400 group-hover:bg-purple-500/5 transition-all">
                                                <b.icon className="w-5 h-5" />
                                            </div>
                                            <div>
                                                <p className="text-base font-black text-foreground/90">{b.type}</p>
                                                <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider">{b.need}</p>
                                            </div>
                                        </div>
                                        <div className="text-right">
                                            <p className="text-[11px] font-bold text-purple-400 uppercase tracking-widest mb-1">Billing</p>
                                            <p className="text-xs font-mono font-black text-foreground/70">{b.pay}</p>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </CardContent>
                    </Card>
                </div>

                {/* Platform Governance & Matching Engine */}
                <Card className="bg-white/[0.02] border-white/[0.05] shadow-2xl overflow-hidden relative group">
                    <div className="absolute inset-0 bg-gradient-to-r from-primary/5 via-transparent to-primary/5 opacity-50" />
                    <CardHeader className="pt-10 pb-8 px-10 text-center relative">
                        <div className="inline-flex items-center gap-3 px-4 py-1.5 rounded-full bg-primary/10 border border-primary/20 text-xs font-black text-primary uppercase tracking-[0.2em] mb-6">
                            <ShieldCheck className="w-4 h-4" /> Platform Governance (AIIGO)
                        </div>
                        <CardTitle className="text-4xl font-black tracking-tight mb-4">Securing the Global Compute Layer</CardTitle>
                        <CardDescription className="text-lg font-medium max-w-3xl mx-auto leading-relaxed">
                            AIIGO operates the decentralized matching engine, maintains dedicated infrastructure for guaranteed SLA, and manages the protocol's economic stability.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="px-10 pb-12 relative">
                        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
                            {[
                                { title: 'Matching Engine', desc: 'Real-time optimization of provider-buyer pairs based on latency and price.', icon: Zap },
                                { title: 'Dedicated SLA', desc: 'High-availability nodes for mission-critical enterprise workloads.', icon: Server },
                                { title: 'Trust & Safety', desc: 'Automated dispute resolution and PoW verification for task accuracy.', icon: Lock },
                                { title: 'Token Economics', desc: 'Stable staking rewards and platform fee (8%) redistribution.', icon: TrendingUp },
                            ].map((item, i) => (
                                <div key={i} className="space-y-4 p-6 rounded-2xl bg-white/[0.03] border border-white/[0.05] hover:bg-white/[0.05] transition-all">
                                    <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center text-primary mb-4">
                                        <item.icon className="w-5 h-5" />
                                    </div>
                                    <h4 className="text-lg font-black">{item.title}</h4>
                                    <p className="text-sm font-medium text-muted-foreground leading-relaxed">{item.desc}</p>
                                </div>
                            ))}
                        </div>
                    </CardContent>
                </Card>

                {/* Popular Resources & Activity Feed */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-10">
                    <Card className="lg:col-span-2 bg-white/[0.02] backdrop-blur-xl border-white/[0.05] shadow-xl gap-0">
                        <CardHeader className="flex flex-row items-center justify-between border-b border-white/[0.05] pt-8 pb-6 px-8">
                            <div>
                                <CardTitle className="text-2xl font-black">Resource Catalog</CardTitle>
                                <CardDescription className="text-sm font-medium mt-1">Global decentralized compute assets</CardDescription>
                            </div>
                            <Button variant="outline" size="sm" className="h-9 px-4 text-xs font-black border-white/10 hover:bg-white/5 rounded-xl transition-all">Browse All</Button>
                        </CardHeader>
                        <CardContent className="p-0">
                            <div className="overflow-x-auto">
                                <table className="w-full">
                                    <thead>
                                        <tr className="border-b border-white/[0.05] text-[10px] font-black text-muted-foreground/60 uppercase tracking-[0.2em] text-left">
                                            <th className="px-8 py-5">Node Identity</th>
                                            <th className="px-8 py-5">Class</th>
                                            <th className="px-8 py-5">Yield Rate</th>
                                            <th className="px-8 py-5">SLA Load</th>
                                            <th className="px-8 py-5 text-right">Match</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-white/[0.05]">
                                        {popularResources.map((res, i) => (
                                            <tr key={i} className="group hover:bg-white/[0.03] transition-all cursor-pointer">
                                                <td className="px-8 py-6">
                                                    <div className="flex items-center gap-4">
                                                        <div className={cn("p-3 rounded-xl bg-white/[0.03] border border-white/[0.05] group-hover:border-primary/20 transition-all shadow-lg", res.color)}>
                                                            <res.icon className="w-5 h-5 stroke-[2]" />
                                                        </div>
                                                        <span className="text-base font-black tracking-tight">{res.name}</span>
                                                    </div>
                                                </td>
                                                <td className="px-8 py-6">
                                                    <div className="px-2.5 py-1 rounded-lg bg-white/[0.05] inline-block text-[11px] font-black text-muted-foreground uppercase">{res.type}</div>
                                                </td>
                                                <td className="px-8 py-6">
                                                    <span className="text-base font-mono font-black text-emerald-400">{res.rate}</span>
                                                </td>
                                                <td className="px-8 py-6">
                                                    <div className="w-32 space-y-2">
                                                        <div className="flex justify-between text-[10px] font-black uppercase text-muted-foreground/70 tracking-tighter">
                                                            <span>Active</span>
                                                            <span>{res.util}%</span>
                                                        </div>
                                                        <Progress value={res.util} className="h-2 bg-white/5" />
                                                    </div>
                                                </td>
                                                <td className="px-8 py-6 text-right">
                                                    <Button variant="ghost" size="sm" className="h-10 px-6 text-xs font-black text-primary group-hover:bg-primary/10 rounded-xl transition-all">
                                                        Deploy
                                                    </Button>
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        </CardContent>
                    </Card>

                    <Card className="bg-white/[0.02] backdrop-blur-xl border-white/[0.05] overflow-hidden flex flex-col shadow-2xl">
                        <CardHeader className="border-b border-white/[0.05] bg-white/[0.02] pt-8 pb-6 px-8">
                            <CardTitle className="text-xl font-black uppercase tracking-tighter">Protocol Events</CardTitle>
                            <CardDescription className="text-sm font-semibold text-primary/80 mt-1">Real-time matching lifecycle</CardDescription>
                        </CardHeader>
                        <CardContent className="p-0 overflow-y-auto max-h-[500px]">
                            <div className="divide-y divide-white/[0.05]">
                                {[
                                    { status: 'PAID', node: 'T1-Frankfurt-01', power: '24 TFLOPS', time: 'Just now', color: 'text-emerald-400 bg-emerald-400/10 border-emerald-400/20' },
                                    { status: 'VERIFYING', node: 'T3-Tokyo-482', power: '4.2 Gbps', time: '2m ago', color: 'text-blue-400 bg-blue-400/10 border-blue-400/20' },
                                    { status: 'EXECUTING', node: 'T2-Oregon-Global', power: '128 Cores', time: '5m ago', color: 'text-purple-400 bg-purple-400/10 border-purple-400/20' },
                                    { status: 'MATCHED', node: 'T3-London-102', power: 'RTX 4090', time: '8m ago', color: 'text-orange-400 bg-orange-400/10 border-orange-400/20' },
                                    { status: 'SUBMITTED', node: 'User-Match-82', power: 'AI Training', time: '12m ago', color: 'text-muted-foreground bg-white/5 border-white/10' },
                                ].map((task, i) => (
                                    <div key={i} className="p-6 hover:bg-white/[0.03] transition-all cursor-pointer group border-l-2 border-transparent hover:border-primary/40">
                                        <div className="flex justify-between items-start mb-3">
                                            <span className={cn("px-2.5 py-0.5 rounded text-[10px] font-black tracking-widest border", task.color.split(' ')[1], task.color.split(' ')[2], task.color.split(' ')[0])}>
                                                {task.status}
                                            </span>
                                            <span className="text-[10px] font-mono font-medium text-muted-foreground/60">{task.time}</span>
                                        </div>
                                        <div className="flex justify-between items-center group-hover:translate-x-1 transition-transform">
                                            <p className="text-sm font-black text-foreground/90">{task.node}</p>
                                            <p className="text-[10px] font-mono font-black bg-white/5 px-3 py-1 rounded-lg border border-white/5 text-muted-foreground">{task.power}</p>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </CardContent>
                        <div className="mt-auto p-4 border-t border-white/[0.05] bg-white/[0.01]">
                            <Button variant="ghost" className="w-full text-[10px] font-black tracking-[0.3em] uppercase text-muted-foreground/50 hover:text-primary transition-all group h-8">
                                <span className="relative flex h-1.5 w-1.5 mr-3">
                                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-40"></span>
                                    <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-primary/70"></span>
                                </span>
                                Stream Active
                            </Button>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    );
};

export default ComputingPower;
