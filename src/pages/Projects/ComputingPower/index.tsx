
import React from 'react';
import { useComputingStore } from './store/useComputingStore';
import MarketplaceView from './MarketplaceView';
import ProviderView from './ProviderView';
import BuyerView from './BuyerView';
import GovernanceView from './GovernanceView';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'; // Assuming Tabs component exists
import { cn } from '@/lib/utils';
import { Home, Zap, ShoppingCart, Scale, Wallet } from 'lucide-react';
import { Button } from '@/components/ui/button';

const ComputingPower: React.FC = () => {
    const { activeTab, setActiveTab, user } = useComputingStore();

    const tabs = [
        { id: 'marketplace', label: 'Marketplace', icon: Home },
        { id: 'provider', label: 'Provider Dashboard', icon: Zap },
        { id: 'buyer', label: 'Buyer Dashboard', icon: ShoppingCart },
        { id: 'governance', label: 'Governance', icon: Scale },
    ];

    return (
        <div className="min-h-screen p-6 font-sans overflow-x-hidden pb-20">
            {/* Ambient Background */}
            <div className="fixed inset-0 pointer-events-none overflow-hidden -z-10">
                <div className="absolute top-[-10%] right-[-10%] w-[60%] h-[60%] bg-blue-600/5 rounded-full blur-[120px] animate-pulse" />
                <div className="absolute bottom-[-10%] left-[-10%] w-[60%] h-[60%] bg-purple-600/5 rounded-full blur-[120px] animate-pulse delay-700" />
            </div>

            <div className="max-w-7xl mx-auto">
                {/* Global Top Bar */}
                <div className="flex justify-between items-center mb-8 sticky top-0 z-50 py-4 bg-background/80 backdrop-blur-md border-b border-white/5 px-2 -mx-2">
                    <div className="flex items-center gap-2">
                        <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary to-purple-600 flex items-center justify-center font-black text-white text-lg">A</div>
                        <span className="font-bold text-lg tracking-tight">AIIGO <span className="text-muted-foreground font-normal">Compute</span></span>
                    </div>

                    <div className="flex items-center gap-4">
                        <div className="hidden md:flex flex-col items-end mr-2">
                            <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-widest">Wallet Balance</span>
                            <span className="font-mono font-bold text-primary">{user.balance.toFixed(4)} ETH</span>
                        </div>
                        <div className="h-10 w-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center">
                            <Wallet className="w-5 h-5 text-muted-foreground" />
                        </div>
                    </div>
                </div>

                {/* Main Navigation */}
                <div className="flex justify-center mb-8">
                    <div className="inline-flex items-center p-1 rounded-xl bg-white/5 border border-white/10">
                        {tabs.map(tab => (
                            <button
                                key={tab.id}
                                onClick={() => setActiveTab(tab.id as any)}
                                className={cn(
                                    "px-6 py-2.5 rounded-lg text-sm font-bold flex items-center gap-2 transition-all duration-300",
                                    activeTab === tab.id
                                        ? "bg-primary text-primary-foreground shadow-lg scale-105"
                                        : "text-muted-foreground hover:text-foreground hover:bg-white/5"
                                )}
                            >
                                <tab.icon className="w-4 h-4" />
                                {tab.label}
                            </button>
                        ))}
                    </div>
                </div>

                {/* Content Area */}
                <div className="min-h-[600px]">
                    {activeTab === 'marketplace' && <MarketplaceView />}
                    {activeTab === 'provider' && <ProviderView />}
                    {activeTab === 'buyer' && <BuyerView />}
                    {activeTab === 'governance' && <GovernanceView />}
                </div>
            </div>
        </div>
    );
};

export default ComputingPower;
