import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Search, Filter, FolderOpen, ExternalLink } from 'lucide-react';

const Projects: React.FC = () => {
    return (
        <div className="min-h-screen p-6 font-sans">
            {/* Background Effects */}
            <div className="fixed inset-0 pointer-events-none overflow-hidden">
                <div className="absolute top-[20%] right-[-10%] w-[40%] h-[40%] bg-emerald-500/5 rounded-full blur-[150px]" />
            </div>

            <div className="relative z-10 max-w-7xl mx-auto space-y-8">
                {/* Header */}
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                    <div>
                        <h1 className="text-3xl font-bold tracking-tight text-foreground">
                            Portfolio Projects
                        </h1>
                        <p className="text-muted-foreground mt-1 text-sm">
                            Track and manage all portfolio companies.
                        </p>
                    </div>
                    <div className="flex items-center gap-3">
                        <div className="relative w-64">
                            <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
                            <Input placeholder="Search projects..." className="pl-8 bg-card/50 border-border/50" />
                        </div>
                        <Button variant="outline" className="gap-2 bg-card/50 border-border/50">
                            <Filter className="h-4 w-4" />
                            Filter
                        </Button>
                    </div>
                </div>

                {/* Projects List */}
                <div className="space-y-4">
                    {[1, 2, 3, 4, 5].map((i) => (
                        <Card key={i} className="bg-card/30 backdrop-blur-xl border-border/50 hover:bg-card/50 transition-all group">
                            <CardContent className="p-6 flex items-center justify-between">
                                <div className="flex items-center gap-4">
                                    <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-slate-800 to-slate-900 flex items-center justify-center border border-border/50">
                                        <FolderOpen className="w-6 h-6 text-muted-foreground group-hover:text-primary transition-colors" />
                                    </div>
                                    <div>
                                        <h3 className="font-semibold text-foreground text-lg">Project Alpha {i}</h3>
                                        <div className="flex items-center gap-3 mt-1">
                                            <span className="text-xs text-muted-foreground">Infrastructure</span>
                                            <div className="w-1 h-1 rounded-full bg-slate-600" />
                                            <span className="text-xs text-emerald-500">Live</span>
                                        </div>
                                    </div>
                                </div>

                                <div className="flex items-center gap-8">
                                    <div className="text-right hidden md:block">
                                        <p className="text-xs text-muted-foreground">Total Invested</p>
                                        <p className="font-mono font-medium text-foreground">$150,000</p>
                                    </div>
                                    <div className="text-right hidden md:block">
                                        <p className="text-xs text-muted-foreground">Current Value</p>
                                        <p className="font-mono font-medium text-emerald-500">$450,000</p>
                                    </div>
                                    <Button variant="ghost" size="icon" className="text-muted-foreground hover:text-foreground">
                                        <ExternalLink className="w-4 h-4" />
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>
                    ))}
                </div>
            </div>
        </div>
    );
};

export default Projects;
