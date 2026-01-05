import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Coins, Inbox } from 'lucide-react';
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@/components/ui/empty';

const Investments: React.FC = () => {
    const investments: any[] = []; // Empty for now as requested
    return (
        <div className="min-h-screen p-6 font-sans">
            {/* Background Effects */}
            <div className="fixed inset-0 pointer-events-none overflow-hidden">
                <div className="absolute top-[20%] left-[-10%] w-[40%] h-[40%] bg-purple-500/5 rounded-full blur-[150px]" />
            </div>

            <div className="relative z-10 max-w-7xl mx-auto space-y-8">
                {/* Header */}
                <div>
                    <h1 className="text-3xl font-bold tracking-tight text-foreground">
                        Investment History
                    </h1>
                    <p className="text-muted-foreground mt-1 text-sm">
                        Detailed record of all capital deployments and returns.
                    </p>
                </div>

                {/* Investment Table/List */}
                <Card className="bg-card/30 backdrop-blur-xl border-border/50">
                    <CardContent className={investments.length > 0 ? "p-0" : "p-12"}>
                        {investments.length > 0 ? (
                            <>
                                <div className="grid grid-cols-12 gap-4 p-4 text-xs font-medium text-muted-foreground border-b border-border/50 uppercase tracking-wider">
                                    <div className="col-span-4">Asset/Project</div>
                                    <div className="col-span-2">Type</div>
                                    <div className="col-span-2">Date</div>
                                    <div className="col-span-2 text-right">Amount</div>
                                    <div className="col-span-2 text-right">Status</div>
                                </div>

                                {investments.map((_, i) => (
                                    <div key={i} className="grid grid-cols-12 gap-4 p-4 items-center hover:bg-white/5 transition-colors border-b border-border/50 last:border-0 cursor-pointer text-sm">
                                        <div className="col-span-4 flex items-center gap-3">
                                            <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center text-primary">
                                                <Coins className="w-4 h-4" />
                                            </div>
                                            <div>
                                                <p className="font-medium text-foreground">Series A - Protocol {i + 1}</p>
                                                <p className="text-xs text-muted-foreground">Equity Round</p>
                                            </div>
                                        </div>
                                        <div className="col-span-2">
                                            <Badge variant="outline" className="bg-purple-500/10 text-purple-500 border-purple-500/20">
                                                Direct
                                            </Badge>
                                        </div>
                                        <div className="col-span-2 text-muted-foreground">
                                            Oct 24, 2024
                                        </div>
                                        <div className="col-span-2 text-right font-mono text-foreground">
                                            $250,000.00
                                        </div>
                                        <div className="col-span-2 flex justify-end">
                                            <Badge className="bg-emerald-500/10 text-emerald-500 hover:bg-emerald-500/20 border-0">
                                                Completed
                                            </Badge>
                                        </div>
                                    </div>
                                ))}
                            </>
                        ) : (
                            <Empty className="border-none bg-transparent">
                                <EmptyHeader>
                                    <EmptyMedia variant="icon">
                                        <Inbox className="w-6 h-6" />
                                    </EmptyMedia>
                                    <EmptyTitle>No investment history</EmptyTitle>
                                    <EmptyDescription>
                                        You haven't made any capital deployments yet.
                                    </EmptyDescription>
                                </EmptyHeader>
                                <EmptyContent>
                                    Detailed record of your future investments will appear here.
                                </EmptyContent>
                            </Empty>
                        )}
                    </CardContent>
                </Card>
            </div>
        </div>
    );
};

export default Investments;
