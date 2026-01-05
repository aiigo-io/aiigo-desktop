import React from 'react';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
    Cpu,
    Zap,
    HardDrive,
    Wifi,
    Smartphone,
    Bike,
    Activity,
    TrendingUp,
    Globe,
    ShieldCheck,
    Box
} from 'lucide-react';
import { Progress } from '@/components/ui/progress';

const ComputingPower: React.FC = () => {
    const resources = [
        { name: 'CPU', total: '128.4 TFLOPS', status: 'Active', icon: Cpu, color: 'text-blue-500', bg: 'bg-blue-500/10' },
        { name: 'GPU', total: '512.8 TFLOPS', status: 'Active', icon: Zap, color: 'text-purple-500', bg: 'bg-purple-500/10' },
        { name: 'Storage', total: '2.5 PB', status: 'Syncing', icon: HardDrive, color: 'text-emerald-500', bg: 'bg-emerald-500/10' },
        { name: 'Network (WiFi)', total: '45.2 Gbps', status: 'Optimized', icon: Wifi, color: 'text-orange-500', bg: 'bg-orange-500/10' },
        { name: 'Mobile Node', total: '15,240 Devices', status: 'Connected', icon: Smartphone, color: 'text-rose-500', bg: 'bg-rose-500/10' },
        { name: 'IoT (E-Bike)', total: '8,420 Nodes', status: 'Mobile', icon: Bike, color: 'text-cyan-500', bg: 'bg-cyan-500/10' },
    ];

    return (
        <div className="min-h-screen p-6 font-sans overflow-x-hidden">
            {/* Ambient Background */}
            <div className="fixed inset-0 pointer-events-none overflow-hidden">
                <div className="absolute top-[-10%] right-[-10%] w-[60%] h-[60%] bg-blue-600/5 rounded-full blur-[120px] animate-pulse" />
                <div className="absolute bottom-[-10%] left-[-10%] w-[60%] h-[60%] bg-purple-600/5 rounded-full blur-[120px] animate-pulse delay-700" />
            </div>

            <div className="relative z-10 max-w-7xl mx-auto space-y-8">
                {/* Header Section */}
                <div className="flex flex-col md:flex-row md:items-end justify-between gap-4">
                    <div className="space-y-2">
                        <div className="inline-flex items-center px-2 py-1 rounded-md bg-primary/10 border border-primary/20 text-[10px] font-bold text-primary uppercase tracking-widest">
                            Global Infrastructure
                        </div>
                        <h1 className="text-4xl font-extrabold tracking-tight text-foreground bg-clip-text text-transparent bg-gradient-to-r from-foreground to-foreground/50">
                            One-Click Computing Power Matching
                        </h1>
                        <p className="text-muted-foreground text-sm max-w-2xl leading-relaxed">
                            A premier application platform for one-click computing power matching: efficiently aggregating CPU, GPU, SSD, WiFi, mobile apps, and e-bike driving data to build a decentralized global computing resource pool, achieving optimal matching and maximizing value for you.
                        </p>
                    </div>
                    <Button className="h-12 px-8 bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg shadow-primary/20 rounded-xl font-bold gap-2 transition-all hover:scale-105 active:scale-95">
                        <Zap className="w-5 h-5 fill-current" />
                        Match Now
                    </Button>
                </div>

                {/* Performance Highlights */}
                <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                    {[
                        { label: 'Total Computing Power', value: '1.24 EH/s', icon: Activity },
                        { label: 'Network Efficiency', value: '99.98%', icon: ShieldCheck },
                        { label: 'Active Matching Nodes', value: '245,820', icon: Box },
                        { label: 'Estimated Daily Yield', value: '12.5% APR', icon: TrendingUp },
                    ].map((item, i) => (
                        <Card key={i} className="bg-card/40 backdrop-blur-xl border-border/50 hover:border-primary/30 transition-colors group">
                            <CardContent className="p-4 flex items-center gap-4">
                                <div className="p-2.5 rounded-lg bg-primary/5 text-primary group-hover:scale-110 transition-transform">
                                    <item.icon className="w-5 h-5" />
                                </div>
                                <div>
                                    <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-tight">{item.label}</p>
                                    <p className="text-lg font-bold text-foreground">{item.value}</p>
                                </div>
                            </CardContent>
                        </Card>
                    ))}
                </div>

                {/* Resource Matrix */}
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {resources.map((res, i) => (
                        <Card key={i} className="relative overflow-hidden bg-card/30 backdrop-blur-md border-border/50 group hover:bg-card/50 transition-all duration-300">
                            <div className={cn("absolute top-0 right-0 w-32 h-32 -mr-8 -mt-8 rounded-full blur-3xl opacity-20 transition-opacity group-hover:opacity-40", res.bg)} />
                            <CardHeader className="pb-2">
                                <div className="flex justify-between items-start">
                                    <div className={cn("p-3 rounded-2xl shadow-inner", res.bg)}>
                                        <res.icon className={cn("w-6 h-6", res.color)} />
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <span className="relative flex h-2 w-2">
                                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                                            <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                                        </span>
                                        <span className="text-[10px] font-bold text-emerald-500 uppercase">{res.status}</span>
                                    </div>
                                </div>
                                <CardTitle className="text-xl mt-4">{res.name}</CardTitle>
                                <CardDescription className="text-foreground/70 font-mono">{res.total}</CardDescription>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                <div className="space-y-1.5">
                                    <div className="flex justify-between text-[10px] font-medium text-muted-foreground uppercase">
                                        <span>Current Load</span>
                                        <span>{70 + (i * 5)}%</span>
                                    </div>
                                    <Progress value={70 + (i * 5)} className="h-1.5 bg-border/30" />
                                </div>
                                <div className="pt-2 flex items-center justify-between">
                                    <div className="flex -space-x-2">
                                        {[1, 2, 3].map(n => (
                                            <div key={n} className="w-6 h-6 rounded-full border-2 border-background bg-muted flex items-center justify-center overflow-hidden">
                                                <div className="w-full h-full bg-gradient-to-br from-primary/40 to-muted" />
                                            </div>
                                        ))}
                                        <div className="w-6 h-6 rounded-full border-2 border-background bg-muted flex items-center justify-center text-[8px] font-bold">
                                            +12
                                        </div>
                                    </div>
                                    <Button variant="ghost" size="sm" className="h-8 text-[11px] font-bold text-primary hover:text-primary hover:bg-primary/5 p-0">
                                        Node Details
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>
                    ))}
                </div>

                {/* Activity Feed & Network Map Placeholder */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    <Card className="lg:col-span-2 bg-card/20 backdrop-blur-xl border-border/50">
                        <CardHeader>
                            <div className="flex items-center justify-between">
                                <div>
                                    <CardTitle className="text-lg">Real-time Matching Feed</CardTitle>
                                    <CardDescription>Global decentralized computing resource allocation</CardDescription>
                                </div>
                                <div className="flex items-center gap-2 text-xs font-mono text-primary animate-pulse">
                                    <Globe className="w-4 h-4" />
                                    SYNCING...
                                </div>
                            </div>
                        </CardHeader>
                        <CardContent>
                            <div className="space-y-1 border-l-2 border-primary/20 ml-2">
                                {[
                                    { time: '12:45:01', action: 'GPU Cluster matched', region: 'Europe/Frankfurt', status: '+1.24 ETH/h' },
                                    { time: '12:44:52', action: 'IoT Node (Mobile) registered', region: 'Asia/Tokyo', status: 'Verifying' },
                                    { time: '12:44:38', action: 'Storage Pool allocation', region: 'North America/CA', status: '+0.45 ETH/h' },
                                    { time: '12:44:15', action: 'CPU Yield optimized', region: 'Asia/Singapore', status: '+0.12 ETH/h' },
                                ].map((log, i) => (
                                    <div key={i} className="relative pl-6 pb-6 last:pb-0">
                                        <div className="absolute left-[-5px] top-1.5 w-2 h-2 rounded-full bg-primary" />
                                        <div className="flex flex-col md:flex-row md:items-center justify-between gap-1 p-3 rounded-lg bg-white/5 hover:bg-white/10 transition-colors cursor-pointer">
                                            <div className="space-y-1">
                                                <div className="flex items-center gap-2">
                                                    <span className="text-[10px] font-mono text-muted-foreground">{log.time}</span>
                                                    <span className="text-sm font-semibold">{log.action}</span>
                                                </div>
                                                <div className="text-[10px] font-medium text-muted-foreground flex items-center gap-1">
                                                    <Globe className="w-3 h-3" /> {log.region}
                                                </div>
                                            </div>
                                            <div className="text-xs font-bold text-emerald-500 font-mono">
                                                {log.status}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </CardContent>
                    </Card>

                    <Card className="bg-gradient-to-br from-blue-600/20 to-purple-600/20 backdrop-blur-xl border-primary/20 flex flex-col items-center justify-center p-8 text-center space-y-4">
                        <div className="relative">
                            <div className="absolute inset-0 bg-primary blur-2xl opacity-20 animate-pulse" />
                            <div className="relative p-6 rounded-full bg-primary/10 border border-primary/30">
                                <Zap className="w-12 h-12 text-primary animate-bounce shadow-2xl" />
                            </div>
                        </div>
                        <div className="space-y-2">
                            <h3 className="text-xl font-bold">READY TO EARN?</h3>
                            <p className="text-sm text-muted-foreground">
                                Connect your computing infrastructure and start earning from the global matching network.
                            </p>
                        </div>
                        <Button className="w-full h-12 rounded-xl font-bold bg-foreground text-background hover:bg-foreground/90 transition-all shadow-xl">
                            Deploy New Node
                        </Button>
                        <p className="text-[10px] text-muted-foreground font-mono">
                            VERIFIED BY AIIGO PROTOCOL 2.0
                        </p>
                    </Card>
                </div>
            </div>
        </div>
    );
};

export default ComputingPower;
