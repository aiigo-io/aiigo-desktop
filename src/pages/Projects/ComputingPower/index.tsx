
import React from 'react';
import MarketplaceView from './MarketplaceView';
import ProviderView from './ProviderView';
import BuyerView from './BuyerView';
import GovernanceView from './GovernanceView';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { cn } from '@/lib/utils';
import { AlertCircle, Home, RefreshCw, ShoppingCart, Scale, Wallet, Zap } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useComputeMarketplace } from './useComputeMarketplace';
import { toast } from 'sonner';

const ComputingPower: React.FC = () => {
    const model = useComputeMarketplace();
    const {
        activeTab,
        setActiveTab,
        wallets,
        selectedWallet,
        selectedWalletId,
        setSelectedWalletId,
        config,
        snapshot,
        error,
        isLoadingWallets,
        isRefreshing,
        pendingActionLabel,
        refreshSnapshot,
    } = model;

    const tabs = [
        { id: 'marketplace', label: 'Marketplace', icon: Home },
        { id: 'provider', label: 'Provider Dashboard', icon: Zap },
        { id: 'buyer', label: 'Buyer Dashboard', icon: ShoppingCart },
        { id: 'governance', label: 'Governance', icon: Scale },
    ];

    const handleRefresh = async () => {
        try {
            await refreshSnapshot();
        } catch (refreshError) {
            toast.error(refreshError instanceof Error ? refreshError.message : String(refreshError));
        }
    };

    return (
        <div className="min-h-screen p-6 font-sans overflow-x-hidden pb-20">
            <div className="fixed inset-0 pointer-events-none overflow-hidden -z-10">
                <div className="absolute top-[-10%] right-[-10%] w-[60%] h-[60%] bg-blue-600/5 rounded-full blur-[120px] animate-pulse" />
                <div className="absolute bottom-[-10%] left-[-10%] w-[60%] h-[60%] bg-purple-600/5 rounded-full blur-[120px] animate-pulse delay-700" />
            </div>

            <div className="max-w-7xl mx-auto">
                <div className="flex justify-between items-center mb-8 sticky top-0 z-50 py-4 bg-background/80 backdrop-blur-md border-b border-white/5 px-2 -mx-2">
                    <div className="flex items-center gap-2">
                        <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary to-purple-600 flex items-center justify-center font-black text-white text-lg">A</div>
                        <span className="font-bold text-lg tracking-tight">AIIGO <span className="text-muted-foreground font-normal">Compute</span></span>
                    </div>

                    <div className="flex items-center gap-3">
                        <div className="hidden md:flex flex-col items-end gap-1 mr-2">
                            <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-widest">Wallet / Chain</span>
                            <div className="flex items-center gap-2">
                                <span className="font-mono text-sm text-primary">{selectedWallet?.address ?? 'No wallet selected'}</span>
                                <Badge variant="outline" className="border-primary/30 text-primary">{config.chainName}</Badge>
                            </div>
                        </div>
                        <div className="w-56 hidden sm:block">
                            <Select value={selectedWalletId ?? undefined} onValueChange={setSelectedWalletId} disabled={wallets.length === 0 || isLoadingWallets}>
                                <SelectTrigger className="border-white/10 bg-white/[0.03]">
                                    <SelectValue placeholder={isLoadingWallets ? 'Loading wallets...' : 'Select wallet'} />
                                </SelectTrigger>
                                <SelectContent>
                                    {wallets.map((wallet) => (
                                        <SelectItem key={wallet.id} value={wallet.id}>{wallet.label}</SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        <Button variant="outline" size="sm" onClick={handleRefresh} disabled={isRefreshing || !selectedWallet}>
                            <RefreshCw className={cn('w-4 h-4 mr-2', isRefreshing && 'animate-spin')} />
                            {isRefreshing ? 'Refreshing' : 'Refresh Chain'}
                        </Button>
                        <div className="h-10 w-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center">
                            <Wallet className="w-5 h-5 text-muted-foreground" />
                        </div>
                    </div>
                </div>

                <div className="mb-6 flex flex-wrap items-center gap-3">
                    {!config.isConfigured && (
                        <div className="flex items-center gap-2 rounded-xl border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-sm text-amber-200">
                            <AlertCircle className="h-4 w-4" />
                            Missing compute contract config: {config.missing.join(', ')}
                        </div>
                    )}
                    {error && (
                        <div className="flex items-center gap-2 rounded-xl border border-rose-500/20 bg-rose-500/10 px-4 py-3 text-sm text-rose-200">
                            <AlertCircle className="h-4 w-4" />
                            {error}
                        </div>
                    )}
                    {pendingActionLabel && (
                        <div className="rounded-xl border border-primary/20 bg-primary/10 px-4 py-3 text-sm text-primary">
                            {pendingActionLabel}
                        </div>
                    )}
                    {snapshot.lastRefreshedAt && (
                        <div className="rounded-xl border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-muted-foreground">
                            Last chain refresh: {new Date(snapshot.lastRefreshedAt).toLocaleString()}
                        </div>
                    )}
                </div>

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

                <div className="min-h-[600px]">
                    {activeTab === 'marketplace' && <MarketplaceView model={model} />}
                    {activeTab === 'provider' && <ProviderView model={model} />}
                    {activeTab === 'buyer' && <BuyerView model={model} />}
                    {activeTab === 'governance' && <GovernanceView model={model} />}
                </div>
            </div>
        </div>
    );
};

export default ComputingPower;
