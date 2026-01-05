import React, { useState, useEffect } from 'react';
import { ArrowUpDown, Settings2, Info, RefreshCw, AlertTriangle, CheckCircle2, Loader2 } from 'lucide-react';
import { useSwap } from '../hooks/useSwap';
import { SUPPORTED_CHAINS, MAX_PRICE_IMPACT_WARNING, MAX_PRICE_IMPACT_BLOCK } from '../constants';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Card, CardContent } from '@/components/ui/card';
import { invoke } from '@tauri-apps/api/core';
import { SlippageSettings } from './SlippageSettings';

interface WalletInfo {
    id: string;
    label: string;
    wallet_type: 'mnemonic' | 'private-key';
    address: string;
    balance: number;
    created_at: string;
    updated_at: string;
}

interface EvmAsset {
    symbol: string;
    name: string;
    decimals: number;
    contract_address: string | null;
}

interface EvmAssetBalance {
    chain: string;
    asset: EvmAsset;
    balance: string;
    balance_float: number;
    usd_price: number;
    usd_value: number;
}

interface EvmChainAssets {
    chain: string;
    chain_id: number;
    total_balance_usd: number;
    assets: EvmAssetBalance[];
}

interface EvmWalletInfo {
    id: string;
    label: string;
    wallet_type: 'mnemonic' | 'private-key';
    address: string;
    chains: EvmChainAssets[];
    total_balance_usd: number;
    created_at: string;
    updated_at: string;
}

interface SwapCardProps {
    wallet?: WalletInfo | null;
}

export const SwapCard: React.FC<SwapCardProps> = ({ wallet }) => {
    const {
        fromChain,
        fromToken,
        toToken,
        amount,
        setAmount,
        fromTokens,
        toTokens,
        handleChainChange,
        setFromToken,
        setToToken,
        flipAssets,
        isValid,
        quote,
        isLoadingQuote,
        quoteError,
        priceImpact,
        slippage,
        setSlippage,
        needsApproval,
        isCheckingApproval,
        approveToken,
        txStatus,
        executeSwap,
    } = useSwap(wallet);

    const [balances, setBalances] = useState<Map<string, number>>(new Map());
    const [isLoadingBalances, setIsLoadingBalances] = useState(false);
    const [showSlippageSettings, setShowSlippageSettings] = useState(false);

    // Load balances when wallet or chain changes
    useEffect(() => {
        if (wallet) {
            loadBalances();
        } else {
            setBalances(new Map());
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [wallet?.id, fromChain.id]);

    const loadBalances = async () => {
        if (!wallet) return;

        setIsLoadingBalances(true);
        try {
            const walletWithBalances = await invoke<EvmWalletInfo>('evm_get_wallet_with_balances', {
                walletId: wallet.id
            });

            // Find the current chain's assets
            const chainAssets = walletWithBalances.chains.find(
                c => c.chain_id === fromChain.id
            );

            if (chainAssets) {
                const newBalances = new Map<string, number>();
                chainAssets.assets.forEach(asset => {
                    newBalances.set(asset.asset.symbol, asset.balance_float);
                });
                setBalances(newBalances);
            } else {
                setBalances(new Map());
            }
        } catch (error) {
            console.error('Error loading balances:', error);
            setBalances(new Map());
        } finally {
            setIsLoadingBalances(false);
        }
    };

    const getBalance = (tokenSymbol: string): string => {
        const balance = balances.get(tokenSymbol);
        if (balance === undefined) {
            return wallet ? '0.00' : '--';
        }
        return balance.toFixed(6);
    };

    const hasInsufficientBalance = (): boolean => {
        if (!wallet || !amount) return false;
        const balance = balances.get(fromToken.symbol);
        if (balance === undefined) return false;
        return parseFloat(amount) > balance;
    };

    const getPriceImpactColor = (): string => {
        if (priceImpact < 1) return 'text-green-500';
        if (priceImpact < 5) return 'text-yellow-500';
        if (priceImpact < 15) return 'text-orange-500';
        return 'text-red-500';
    };

    const getButtonText = (): string => {
        if (!wallet) return 'Connect Wallet';
        if (!amount) return 'Enter Amount';
        if (!isValid) return 'Invalid Pair';
        if (hasInsufficientBalance()) return 'Insufficient Balance';
        if (isCheckingApproval) return 'Checking Approval...';
        if (needsApproval) return `Approve ${fromToken.symbol}`;
        if (txStatus.status === 'approving') return 'Approving...';
        if (txStatus.status === 'swapping') return 'Swapping...';
        return 'Swap';
    };

    const isButtonDisabled = (): boolean => {
        if (!wallet || !isValid || !amount) return true;
        if (hasInsufficientBalance()) return true;
        if (isCheckingApproval || isLoadingQuote) return true;
        if (txStatus.status === 'approving' || txStatus.status === 'swapping') return true;
        if (priceImpact > MAX_PRICE_IMPACT_BLOCK) return true;
        return false;
    };

    const handleButtonClick = async () => {
        if (!wallet) return;

        try {
            if (needsApproval) {
                await approveToken();
            } else {
                await executeSwap();
            }
        } catch (error) {
            console.error('Transaction error:', error);
        }
    };

    const formatOutputAmount = (): string => {
        if (isLoadingQuote) return '...';
        if (!quote) return '0.0';

        // API returns outAmount as a string with decimals (e.g., "100148000000" for USDC with 6 decimals)
        // We need to divide by 10^decimals to get the actual amount
        const rawAmount = parseFloat(quote.outAmount);
        const decimals = quote.outToken.decimals;
        const actualAmount = rawAmount / Math.pow(10, decimals);

        return actualAmount.toFixed(6);
    };

    return (
        <>
            <Card className="w-full max-w-[480px] mx-auto shadow-2xl">
                <CardContent className="p-6 space-y-4">
                    <div className="flex items-center justify-between mb-2">
                        <div className="flex items-center gap-3">
                            <h2 className="text-lg font-semibold text-foreground">Swap</h2>
                            <div className="h-4 w-px bg-border" />
                            <Select
                                value={fromChain.id.toString()}
                                onValueChange={(val) => handleChainChange(SUPPORTED_CHAINS.find(c => c.id.toString() === val)!)}
                            >
                                <SelectTrigger className="h-7 border-none bg-muted/50 hover:bg-muted transition-colors text-xs font-medium px-2 rounded-full">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {SUPPORTED_CHAINS.map((chain) => (
                                        <SelectItem key={chain.id} value={chain.id.toString()}>
                                            <div className="flex items-center gap-2">
                                                <img src={chain.logoURI} alt={chain.name} className="size-3.5 rounded-full" />
                                                <span>{chain.name}</span>
                                            </div>
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        <Button
                            variant="ghost"
                            size="icon"
                            className="text-muted-foreground hover:text-foreground"
                            onClick={() => setShowSlippageSettings(true)}
                        >
                            <Settings2 className="size-4" />
                        </Button>
                    </div>

                    {/* From Section */}
                    <div className="space-y-2 p-4 rounded-2xl bg-muted/30 border border-transparent hover:border-border transition-all">
                        <div className="flex items-center justify-between text-xs font-medium text-muted-foreground">
                            <span>From</span>
                            <div className="flex items-center gap-2">
                                <span>Balance: {isLoadingBalances ? '...' : getBalance(fromToken.symbol)}</span>
                                {wallet && balances.get(fromToken.symbol) !== undefined && balances.get(fromToken.symbol)! > 0 && (
                                    <button
                                        onClick={() => setAmount(getBalance(fromToken.symbol))}
                                        className="px-2 py-0.5 text-xs bg-primary/10 text-primary hover:bg-primary/20 rounded transition-colors font-semibold cursor-pointer"
                                        title="Use max balance"
                                    >
                                        MAX
                                    </button>
                                )}
                                {wallet && (
                                    <button
                                        onClick={loadBalances}
                                        className="text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                                        title="Refresh balance"
                                        disabled={isLoadingBalances}
                                    >
                                        <RefreshCw className={`size-3 ${isLoadingBalances ? 'animate-spin' : ''}`} />
                                    </button>
                                )}
                            </div>
                        </div>

                        <div className="flex items-center gap-3">
                            <Select
                                value={fromToken.symbol}
                                onValueChange={(val) => setFromToken(fromTokens.find(t => t.symbol === val)!)}
                            >
                                <SelectTrigger className="w-[130px] shrink-0 h-10 border-none bg-card shadow-sm hover:bg-accent/50 transition-all">
                                    <SelectValue>
                                        <div className="flex items-center gap-2">
                                            <img src={fromToken.logoURI} alt={fromToken.symbol} className="size-5 rounded-full" />
                                            <span className="font-bold">{fromToken.symbol}</span>
                                        </div>
                                    </SelectValue>
                                </SelectTrigger>
                                <SelectContent>
                                    {fromTokens.map((token) => (
                                        <SelectItem key={token.symbol} value={token.symbol}>
                                            <div className="flex items-center gap-2">
                                                <img src={token.logoURI} alt={token.symbol} className="size-5 rounded-full" />
                                                <span>{token.symbol}</span>
                                            </div>
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                            <Input
                                type="number"
                                placeholder="0.0"
                                value={amount}
                                onChange={(e) => setAmount(e.target.value)}
                                className="flex-1 border-none bg-transparent text-2xl font-semibold text-right focus-visible:ring-0 p-0 text-foreground"
                            />
                        </div>
                    </div>

                    {/* Flip Button */}
                    <div className="relative h-2">
                        <Button
                            variant="outline"
                            size="icon"
                            onClick={flipAssets}
                            className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 size-8 rounded-xl bg-card border-border shadow-sm hover:bg-accent hover:scale-110 transition-all z-10"
                        >
                            <ArrowUpDown className="size-4 text-primary" />
                        </Button>
                    </div>

                    {/* To Section */}
                    <div className="space-y-2 p-4 rounded-2xl bg-muted/30 border border-transparent hover:border-border transition-all">
                        <div className="flex items-center justify-between text-xs font-medium text-muted-foreground">
                            <span>To</span>
                            <span>Balance: {isLoadingBalances ? '...' : getBalance(toToken.symbol)}</span>
                        </div>

                        <div className="flex items-center gap-3">
                            <Select
                                value={toToken.symbol}
                                onValueChange={(val) => setToToken(toTokens.find(t => t.symbol === val)!)}
                            >
                                <SelectTrigger className="w-[130px] shrink-0 h-10 border-none bg-card shadow-sm hover:bg-accent/50 transition-all">
                                    <SelectValue>
                                        <div className="flex items-center gap-2">
                                            <img src={toToken.logoURI} alt={toToken.symbol} className="size-5 rounded-full" />
                                            <span className="font-bold">{toToken.symbol}</span>
                                        </div>
                                    </SelectValue>
                                </SelectTrigger>
                                <SelectContent>
                                    {toTokens.map((token) => (
                                        <SelectItem key={token.symbol} value={token.symbol}>
                                            <div className="flex items-center gap-2">
                                                <img src={token.logoURI} alt={token.symbol} className="size-5 rounded-full" />
                                                <span>{token.symbol}</span>
                                            </div>
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                            <div className="flex-1 text-right text-2xl font-semibold text-foreground">
                                {formatOutputAmount()}
                            </div>
                        </div>
                    </div>

                    {/* Price Impact Warning */}
                    {priceImpact > MAX_PRICE_IMPACT_WARNING && quote && (
                        <div className={`flex items-start gap-2 p-3 rounded-xl ${priceImpact > MAX_PRICE_IMPACT_BLOCK
                            ? 'bg-destructive/10 border border-destructive/20'
                            : 'bg-orange-500/10 border border-orange-500/20'
                            }`}>
                            <AlertTriangle className={`size-4 mt-0.5 ${priceImpact > MAX_PRICE_IMPACT_BLOCK ? 'text-destructive' : 'text-orange-500'
                                }`} />
                            <div className="flex-1 text-xs">
                                <p className={`font-semibold ${priceImpact > MAX_PRICE_IMPACT_BLOCK ? 'text-destructive' : 'text-orange-500'
                                    }`}>
                                    {priceImpact > MAX_PRICE_IMPACT_BLOCK ? 'Price Impact Too High' : 'High Price Impact'}
                                </p>
                                <p className={`${priceImpact > MAX_PRICE_IMPACT_BLOCK ? 'text-destructive/80' : 'text-orange-500/80'
                                    }`}>
                                    This swap has a {priceImpact.toFixed(2)}% price impact.
                                    {priceImpact > MAX_PRICE_IMPACT_BLOCK && ' Transaction blocked for your protection.'}
                                </p>
                            </div>
                        </div>
                    )}

                    {/* Quote Error */}
                    {quoteError && (
                        <div className="flex items-start gap-2 p-3 rounded-xl bg-destructive/10 border border-destructive/20">
                            <AlertTriangle className="size-4 mt-0.5 text-destructive" />
                            <div className="flex-1 text-xs">
                                <p className="font-semibold text-destructive">Quote Error</p>
                                <p className="text-destructive/80">{quoteError}</p>
                            </div>
                        </div>
                    )}

                    {/* Transaction Status */}
                    {txStatus.status === 'success' && txStatus.hash && (
                        <div className="flex items-start gap-2 p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
                            <CheckCircle2 className="size-4 mt-0.5 text-emerald-500" />
                            <div className="flex-1 text-xs">
                                <p className="font-semibold text-emerald-500">Transaction Successful</p>
                                <p className="text-emerald-500/80 truncate">Hash: {txStatus.hash}</p>
                            </div>
                        </div>
                    )}

                    {txStatus.status === 'error' && txStatus.error && (
                        <div className="flex items-start gap-2 p-3 rounded-xl bg-destructive/10 border border-destructive/20">
                            <AlertTriangle className="size-4 mt-0.5 text-destructive" />
                            <div className="flex-1 text-xs">
                                <p className="font-semibold text-destructive">Transaction Failed</p>
                                <p className="text-destructive/80">{txStatus.error}</p>
                            </div>
                        </div>
                    )}

                    {/* Quote Details */}
                    {amount && quote && !quoteError && (
                        <div className="px-1 py-2 space-y-2">
                            <div className="flex items-center justify-between text-xs">
                                <span className="text-muted-foreground flex items-center gap-1">
                                    Price Impact <Info className="size-3" />
                                </span>
                                <span className={`font-medium ${getPriceImpactColor()}`}>
                                    {priceImpact < 0.01 ? '< 0.01' : priceImpact.toFixed(2)}%
                                </span>
                            </div>
                            <div className="flex items-center justify-between text-xs">
                                <span className="text-muted-foreground">Slippage Tolerance</span>
                                <span className="text-foreground font-medium">{slippage}%</span>
                            </div>
                            <div className="flex items-center justify-between text-xs">
                                <span className="text-muted-foreground">Route</span>
                                <span className="text-foreground font-medium">OpenOcean Optimal</span>
                            </div>
                            {quote.estimatedGas && (
                                <div className="flex items-center justify-between text-xs">
                                    <span className="text-muted-foreground">Est. Gas</span>
                                    <span className="text-foreground font-medium">{quote.estimatedGas.toLocaleString()}</span>
                                </div>
                            )}
                        </div>
                    )}

                    {/* Swap Button */}
                    <Button
                        className="w-full h-12 rounded-xl text-lg shadow-lg active:scale-[0.98] transition-all"
                        disabled={isButtonDisabled()}
                        onClick={handleButtonClick}
                    >
                        {(txStatus.status === 'approving' || txStatus.status === 'swapping' || isLoadingQuote) ? (
                            <div className="flex items-center gap-2">
                                <Loader2 className="size-4 animate-spin" />
                                {getButtonText()}
                            </div>
                        ) : (
                            getButtonText()
                        )}
                    </Button>
                </CardContent>
            </Card>

            {/* Slippage Settings Modal */}
            {showSlippageSettings && (
                <SlippageSettings
                    slippage={slippage}
                    onSlippageChange={setSlippage}
                    onClose={() => setShowSlippageSettings(false)}
                />
            )}
        </>
    );
};
