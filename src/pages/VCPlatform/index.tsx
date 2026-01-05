import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { TrendingUp, Users, Wallet, ArrowUpRight, Target, Clock, Inbox } from 'lucide-react';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Empty,
    EmptyHeader,
    EmptyTitle,
    EmptyDescription,
    EmptyMedia,
} from '@/components/ui/empty';

const VCPlatform: React.FC = () => {
    const [isLoading, setIsLoading] = useState(true);
    const [deals, setDeals] = useState<number[]>([]);
    const [stats, setStats] = useState<{
        totalDeployed: number;
        activePortfolio: number;
        dealFlow: number;
        growth: number | null;
    } | null>(null);

    useEffect(() => {
        // Simulate initial loading
        const timer = setTimeout(() => {
            setIsLoading(false);
            // Simulate no data currently
            setDeals([]);
            setStats({
                totalDeployed: 0,
                activePortfolio: 0,
                dealFlow: 0,
                growth: null
            });
        }, 1500);

        return () => clearTimeout(timer);
    }, []);

    return (
        <div className="min-h-screen p-6 font-sans">
            {/* Background Effects */}
            <div className="fixed inset-0 pointer-events-none overflow-hidden">
                <div className="absolute top-[-20%] right-[-10%] w-[50%] h-[50%] bg-purple-500/5 rounded-full blur-[150px]" />
                <div className="absolute bottom-[-20%] left-[-10%] w-[50%] h-[50%] bg-blue-500/5 rounded-full blur-[150px]" />
            </div>

            <div className="relative z-10 max-w-7xl mx-auto space-y-8">
                {/* Header */}
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-3xl font-bold tracking-tight text-foreground">
                            Venture Platform
                        </h1>
                        <p className="text-muted-foreground mt-1 text-sm">
                            Manage investments, track portfolio performance, and discover new opportunities.
                        </p>
                    </div>
                    <Button className="bg-primary/10 text-primary hover:bg-primary/20 border border-primary/20">
                        <Target className="mr-2 h-4 w-4" />
                        New Deal
                    </Button>
                </div>

                {/* Stats Grid */}
                <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                    <Card className="bg-card/50 backdrop-blur-xl border-border/50">
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium text-muted-foreground">
                                Total Deployed
                            </CardTitle>
                            <Wallet className="h-4 w-4 text-primary" />
                        </CardHeader>
                        <CardContent>
                            {isLoading ? (
                                <div className="space-y-2">
                                    <Skeleton className="h-8 w-24" />
                                    <Skeleton className="h-4 w-32" />
                                </div>
                            ) : (
                                <>
                                    <div className="text-2xl font-bold text-foreground">
                                        {stats?.totalDeployed ? `$${(stats.totalDeployed / 1000000).toFixed(1)}M` : '$0.00'}
                                    </div>
                                    <p className="text-xs text-muted-foreground mt-1 flex items-center">
                                        {stats?.growth ? (
                                            <>
                                                <ArrowUpRight className="h-3 w-3 text-emerald-500 mr-1" />
                                                <span className="text-emerald-500 font-medium">+{stats.growth}%</span>
                                                <span className="ml-1">from last month</span>
                                            </>
                                        ) : (
                                            <span className="text-muted-foreground/50 italic text-[10px]">No recent activity</span>
                                        )}
                                    </p>
                                </>
                            )}
                        </CardContent>
                    </Card>
                    <Card className="bg-card/50 backdrop-blur-xl border-border/50">
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium text-muted-foreground">
                                Active Portfolio
                            </CardTitle>
                            <TrendingUp className="h-4 w-4 text-purple-500" />
                        </CardHeader>
                        <CardContent>
                            {isLoading ? (
                                <div className="space-y-2">
                                    <Skeleton className="h-8 w-12" />
                                    <Skeleton className="h-4 w-40" />
                                </div>
                            ) : (
                                <>
                                    <div className="text-2xl font-bold text-foreground">{stats?.activePortfolio || '0'}</div>
                                    <p className="text-xs text-muted-foreground mt-1">
                                        {stats?.activePortfolio ? 'Active startups in portfolio' : 'No active investments'}
                                    </p>
                                </>
                            )}
                        </CardContent>
                    </Card>
                    <Card className="bg-card/50 backdrop-blur-xl border-border/50">
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium text-muted-foreground">
                                Deal Flow
                            </CardTitle>
                            <Users className="h-4 w-4 text-blue-500" />
                        </CardHeader>
                        <CardContent>
                            {isLoading ? (
                                <div className="space-y-2">
                                    <Skeleton className="h-8 w-12" />
                                    <Skeleton className="h-4 w-40" />
                                </div>
                            ) : (
                                <>
                                    <div className="text-2xl font-bold text-foreground">{stats?.dealFlow || '0'}</div>
                                    <p className="text-xs text-muted-foreground mt-1">
                                        {stats?.dealFlow ? 'New opportunities this week' : 'No new opportunities currently'}
                                    </p>
                                </>
                            )}
                        </CardContent>
                    </Card>
                </div>

                {/* Recent Deals Section */}
                <div className="space-y-4">
                    <div className="flex items-center justify-between">
                        <h2 className="text-xl font-semibold text-foreground">Active Deals</h2>
                        <Button variant="ghost" size="sm" className="text-muted-foreground hover:text-foreground">
                            View All
                        </Button>
                    </div>

                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                        {isLoading ? (
                            [1, 2, 3].map((i) => (
                                <Card key={i} className="bg-card/30 backdrop-blur-xl border-border/50">
                                    <CardContent className="p-6">
                                        <div className="flex items-start justify-between mb-4">
                                            <div className="flex items-center gap-3">
                                                <Skeleton className="w-10 h-10 rounded-full" />
                                                <div className="space-y-1">
                                                    <Skeleton className="h-4 w-24" />
                                                    <Skeleton className="h-3 w-16" />
                                                </div>
                                            </div>
                                            <Skeleton className="h-5 w-16 rounded-full" />
                                        </div>

                                        <div className="space-y-3 mb-4">
                                            <div className="flex justify-between">
                                                <Skeleton className="h-3 w-16" />
                                                <Skeleton className="h-3 w-12" />
                                            </div>
                                            <div className="flex justify-between">
                                                <Skeleton className="h-3 w-16" />
                                                <Skeleton className="h-3 w-12" />
                                            </div>
                                        </div>

                                        <div className="pt-4 border-t border-border/50 flex items-center justify-between">
                                            <Skeleton className="h-3 w-24" />
                                            <Skeleton className="h-3 w-16" />
                                        </div>
                                    </CardContent>
                                </Card>
                            ))
                        ) : deals.length > 0 ? (
                            deals.map((i) => (
                                <Card key={i} className="bg-card/30 backdrop-blur-xl border-border/50 hover:bg-card/50 transition-all cursor-pointer group">
                                    <CardContent className="p-6">
                                        <div className="flex items-start justify-between mb-4">
                                            <div className="flex items-center gap-3">
                                                <div className="w-10 h-10 rounded-full bg-gradient-to-br from-primary/20 to-purple-500/20 flex items-center justify-center">
                                                    <Target className="w-5 h-5 text-primary" />
                                                </div>
                                                <div>
                                                    <h3 className="font-semibold text-foreground group-hover:text-primary transition-colors">Protocol {i}</h3>
                                                    <p className="text-xs text-muted-foreground">DeFi Infrastructure</p>
                                                </div>
                                            </div>
                                            <div className="px-2 py-1 rounded-full bg-blue-500/10 border border-blue-500/20 text-[10px] font-medium text-blue-500">
                                                Seed Round
                                            </div>
                                        </div>

                                        <div className="space-y-2 mb-4">
                                            <div className="flex justify-between text-sm">
                                                <span className="text-muted-foreground">Raise Amount</span>
                                                <span className="text-foreground font-medium">$2.5M</span>
                                            </div>
                                            <div className="flex justify-between text-sm">
                                                <span className="text-muted-foreground">Valuation</span>
                                                <span className="text-foreground font-medium">$25M</span>
                                            </div>
                                        </div>

                                        <div className="pt-4 border-t border-border/50 flex items-center justify-between text-xs text-muted-foreground">
                                            <div className="flex items-center">
                                                <Clock className="w-3 h-3 mr-1" />
                                                Closes in 5 days
                                            </div>
                                            <span className="text-primary hover:underline">View Details</span>
                                        </div>
                                    </CardContent>
                                </Card>
                            ))
                        ) : (
                            <div className="col-span-full py-12">
                                <Empty className="bg-card/20 backdrop-blur-sm border-dashed border-border/50">
                                    <EmptyMedia variant="icon">
                                        <Inbox className="h-8 w-8 text-muted-foreground" />
                                    </EmptyMedia>
                                    <EmptyHeader>
                                        <EmptyTitle className="text-foreground/80">No active deals</EmptyTitle>
                                        <EmptyDescription className="max-w-[280px] mx-auto">
                                            There are currently no active deal opportunities. Check back later or discover new startups.
                                        </EmptyDescription>
                                    </EmptyHeader>
                                </Empty>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
};

export default VCPlatform;
