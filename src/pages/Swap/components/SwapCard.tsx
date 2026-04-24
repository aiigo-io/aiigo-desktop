import React, { useState, useEffect } from 'react';
import { ArrowUpDown, Settings2, Info, RefreshCw, AlertTriangle, CheckCircle2, Loader2 } from 'lucide-react';
import RecoveryPanel from '@/components/common/RecoveryPanel';
import { useSecuritySession } from '@/components/common/SecuritySession';
import { UnlockGate } from '@/components/common/UnlockGate';
import { parseSecurityError } from '@/lib/security';
import { useSwap } from '../hooks/useSwap';
import { SwapActionIntent } from '../types';
import { SUPPORTED_CHAINS, MAX_PRICE_IMPACT_WARNING, MAX_PRICE_IMPACT_BLOCK } from '../constants';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Card, CardContent } from '@/components/ui/card';
import { invoke, isTauriRuntimeAvailable, isTauriUnavailableError, TAURI_UNAVAILABLE_MESSAGE } from '@/lib/tauri';
import { SlippageSettings } from './SlippageSettings';
import {
    EvmWalletBalancesResponse,
    FreshnessMetadata,
    formatFreshnessLabel,
    getChainFreshnessDescription,
    getEvmChainAssets,
    getFreshnessBadgeClass,
    getWalletSyncBanner,
} from '@/lib/evm-wallet';
import { describeWalletRecovery, type WalletRecoveryFlow, type WalletRecoveryGuidance } from '@/lib/wallet-recovery';

interface WalletInfo {
    id: string;
    label: string;
    wallet_type: 'mnemonic' | 'private-key';
    address: string;
    balance: number;
    created_at: string;
    updated_at: string;
}

interface SwapCardProps {
    wallet?: WalletInfo | null;
}

export const SwapCard: React.FC<SwapCardProps> = ({ wallet }) => {
    const { requestUnlock } = useSecuritySession();
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
        prepareApproveAction,
        submitApproveAction,
        txStatus,
        prepareSwapExecutionAction,
        submitSwapExecutionAction,
    } = useSwap(wallet);

    const [balances, setBalances] = useState<Map<string, number>>(new Map());
    const [isLoadingBalances, setIsLoadingBalances] = useState(false);
    const [showSlippageSettings, setShowSlippageSettings] = useState(false);
    const [chainFreshness, setChainFreshness] = useState<FreshnessMetadata | null>(null);
    const [walletResponse, setWalletResponse] = useState<EvmWalletBalancesResponse | null>(null);
    const [pendingAction, setPendingAction] = useState<SwapActionIntent | null>(null);
    const [isPreparingAction, setIsPreparingAction] = useState(false);
    const [isSubmittingAction, setIsSubmittingAction] = useState(false);
    const [lastActionFlow, setLastActionFlow] = useState<WalletRecoveryFlow>('send-asset');
    const [flowRecovery, setFlowRecovery] = useState<WalletRecoveryGuidance | null>(null);

    // Load balances when wallet or chain changes
    useEffect(() => {
        if (wallet) {
            loadBalances();
        } else {
            setBalances(new Map());
            setChainFreshness(null);
            setWalletResponse(null);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [wallet?.id, fromChain.id]);

    const loadBalances = async () => {
        if (!wallet) return;

        if (!isTauriRuntimeAvailable()) {
            setBalances(new Map());
            setChainFreshness({
                status: 'unavailable',
                updated_at: null,
                failed_sources: ['tauri-runtime'],
            });
            setWalletResponse(null);
            setIsLoadingBalances(false);
            return;
        }

        setIsLoadingBalances(true);
        try {
            const walletWithBalances = await invoke<EvmWalletBalancesResponse>('query_evm_wallet_balances', {
                walletId: wallet.id
            });
            setWalletResponse(walletWithBalances);

            // Find the current chain's assets
            const chainAssets = getEvmChainAssets(walletWithBalances, fromChain.id);

            if (chainAssets) {
                const newBalances = new Map<string, number>();
                chainAssets.assets.forEach(asset => {
                    newBalances.set(asset.asset.symbol, asset.balance_float);
                });
                setBalances(newBalances);
                setChainFreshness(chainAssets.freshness);
            } else {
                setBalances(new Map());
                setChainFreshness({
                    status: 'unavailable',
                    updated_at: null,
                    failed_sources: [fromChain.name.toLowerCase()],
                });
            }
        } catch (error) {
            if (!isTauriUnavailableError(error)) {
                console.error('Error loading balances:', error);
            }
            setBalances(new Map());
            setChainFreshness({
                status: 'unavailable',
                updated_at: null,
                failed_sources: [fromChain.name.toLowerCase()],
            });
        } finally {
            setIsLoadingBalances(false);
        }
    };

    const getBalance = (tokenSymbol: string): string => {
        if (chainFreshness?.status === 'unavailable') {
            return 'Unavailable';
        }

        const balance = balances.get(tokenSymbol);
        if (balance === undefined) {
            return wallet ? '0.00' : '--';
        }
        return balance.toFixed(6);
    };

    const getNumericBalance = (tokenSymbol: string): number | undefined => balances.get(tokenSymbol);

    const isChainUnavailable = chainFreshness?.status === 'unavailable';
    const chainStatusBanner = chainFreshness
        ? {
            label: formatFreshnessLabel(chainFreshness.status),
            description: getChainFreshnessDescription(chainFreshness),
            className: getFreshnessBadgeClass(chainFreshness.status),
        }
        : null;
    const walletSyncBanner = walletResponse ? getWalletSyncBanner(walletResponse) : null;

    const hasInsufficientBalance = (): boolean => {
        if (!wallet || !amount) return false;
        if (isChainUnavailable) return true;
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
        if (isChainUnavailable) return 'Chain Unavailable';
        if (!amount) return 'Enter Amount';
        if (!isValid) return 'Invalid Pair';
        if (hasInsufficientBalance()) return 'Insufficient Balance';
        if (isCheckingApproval) return 'Checking Approval...';
        if (isPreparingAction) return needsApproval ? 'Preparing Approval...' : 'Preparing Review...';
        if (isSubmittingAction) return pendingAction?.uiActionLabel ?? 'Submitting...';
        if (needsApproval) return `Approve ${fromToken.symbol}`;
        if (txStatus.status === 'approving') return 'Approving...';
        if (txStatus.status === 'swapping') return 'Swapping...';
        return 'Swap';
    };

    const isButtonDisabled = (): boolean => {
        if (!wallet || !isValid || !amount) return true;
        if (isChainUnavailable) return true;
        if (hasInsufficientBalance()) return true;
        if (isCheckingApproval || isLoadingQuote) return true;
        if (isPreparingAction || isSubmittingAction) return true;
        if (txStatus.status === 'approving' || txStatus.status === 'swapping') return true;
        if (priceImpact > MAX_PRICE_IMPACT_BLOCK) return true;
        return false;
    };

    const handleButtonClick = async () => {
        if (!wallet) return;

        const currentFlow: WalletRecoveryFlow = needsApproval ? 'approve-allowance' : 'send-asset';

        try {
            setIsPreparingAction(true);
            setFlowRecovery(null);
            setLastActionFlow(currentFlow);
            const intent = needsApproval
                ? await prepareApproveAction()
                : await prepareSwapExecutionAction();
            setPendingAction(intent);
        } catch (error) {
            console.error('Transaction error:', error);
            if (isTauriUnavailableError(error)) {
                return;
            }

            const securityError = parseSecurityError(error);
            if (securityError === 'locked' || securityError === 'expired') {
                await requestUnlock({
                    prompt: needsApproval ? `Unlock to approve ${fromToken.symbol}.` : 'Unlock to continue swap.',
                    reason: securityError,
                    onUnlockSuccess: handleButtonClick,
                });
            } else {
                setFlowRecovery(describeWalletRecovery(currentFlow, error, { chainFamily: 'evm' }));
            }
        } finally {
            setIsPreparingAction(false);
        }
    };

    const handleConfirmAction = async () => {
        if (!pendingAction) return;

        const currentFlow: WalletRecoveryFlow = pendingAction.kind === 'swap-approve' ? 'approve-allowance' : 'send-asset';

        try {
            setIsSubmittingAction(true);
            setFlowRecovery(null);
            setLastActionFlow(currentFlow);
            if (pendingAction.kind === 'swap-approve') {
                await submitApproveAction(pendingAction);
            } else {
                await submitSwapExecutionAction(pendingAction);
            }
            setPendingAction(null);
        } catch (error) {
            console.error('Confirmation error:', error);
            if (isTauriUnavailableError(error)) {
                setPendingAction(null);
                return;
            }

            const securityError = parseSecurityError(error);
            if (securityError === 'locked' || securityError === 'expired') {
                setFlowRecovery(describeWalletRecovery(lastActionFlow, error, { chainFamily: 'evm' }));
                setPendingAction(null);
                await requestUnlock({
                    prompt: pendingAction.kind === 'swap-approve'
                        ? `Unlock to approve ${fromToken.symbol}.`
                        : 'Unlock to execute swap.',
                    reason: securityError,
                    onUnlockSuccess: handleButtonClick,
                });
                return;
            }

            setFlowRecovery(describeWalletRecovery(currentFlow, error, { chainFamily: 'evm' }));
            setPendingAction(null);
        } finally {
            setIsSubmittingAction(false);
        }
    };

    const quoteRecovery = quoteError
        ? describeWalletRecovery('send-asset', quoteError, { chainFamily: 'evm' })
        : null;
    const txRecovery = txStatus.status === 'error' && txStatus.error
        ? describeWalletRecovery(lastActionFlow, txStatus.error, { chainFamily: 'evm' })
        : null;

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

                    {chainStatusBanner && chainFreshness?.status !== 'fresh' && (
                        <div className={`rounded-xl border px-3 py-2 text-xs ${chainStatusBanner.className}`}>
                            <div className="font-semibold uppercase tracking-wide">{chainStatusBanner.label}</div>
                            <div className="mt-1 leading-relaxed">
                                {isTauriRuntimeAvailable() ? chainStatusBanner.description : TAURI_UNAVAILABLE_MESSAGE}
                            </div>
                        </div>
                    )}

                    {walletSyncBanner && (
                        <div className={`rounded-xl border px-3 py-2 text-xs ${walletSyncBanner.className}`}>
                            <div className="font-semibold uppercase tracking-wide">{walletSyncBanner.label}</div>
                            <div className="mt-1 leading-relaxed">{walletSyncBanner.description}</div>
                        </div>
                    )}

                    {/* From Section */}
                    <div className="space-y-2 p-4 rounded-2xl bg-muted/30 border border-transparent hover:border-border transition-all">
                        <div className="flex items-center justify-between text-xs font-medium text-muted-foreground">
                            <span>From</span>
                            <div className="flex items-center gap-2">
                                <span>Balance: {isLoadingBalances ? '...' : getBalance(fromToken.symbol)}</span>
                                {chainFreshness && (
                                    <span className={`rounded-full border px-2 py-0.5 text-[10px] uppercase ${getFreshnessBadgeClass(chainFreshness.status)}`}>
                                        {formatFreshnessLabel(chainFreshness.status)}
                                    </span>
                                )}
                                {wallet && getNumericBalance(fromToken.symbol) !== undefined && getNumericBalance(fromToken.symbol)! > 0 && !isChainUnavailable && (
                                    <button
                                        onClick={() => setAmount(getNumericBalance(fromToken.symbol)!.toFixed(6))}
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
                        <RecoveryPanel guidance={quoteRecovery ?? describeWalletRecovery('send-asset', quoteError, { chainFamily: 'evm' })} />
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

                    {flowRecovery && <RecoveryPanel guidance={flowRecovery} />}

                    {txStatus.status === 'error' && txStatus.error && (
                        <RecoveryPanel guidance={txRecovery ?? describeWalletRecovery(lastActionFlow, txStatus.error, { chainFamily: 'evm' })} />
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
                    <UnlockGate
                        className="w-full"
                        prompt="Unlock to approve or swap"
                        onUnlockSuccess={handleButtonClick}
                    >
                        <Button
                            className="w-full h-12 rounded-xl text-lg shadow-lg active:scale-[0.98] transition-all"
                            disabled={isButtonDisabled()}
                            onClick={handleButtonClick}
                        >
                            {(txStatus.status === 'approving' || txStatus.status === 'swapping' || isLoadingQuote || isPreparingAction || isSubmittingAction) ? (
                                <div className="flex items-center gap-2">
                                    <Loader2 className="size-4 animate-spin" />
                                    {getButtonText()}
                                </div>
                            ) : (
                                getButtonText()
                            )}
                        </Button>
                    </UnlockGate>
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

            <Dialog open={Boolean(pendingAction)} onOpenChange={(open) => !open && setPendingAction(null)}>
                <DialogContent className="sm:max-w-[560px]">
                    <DialogHeader className="space-y-2">
                        <DialogTitle>{pendingAction?.uiActionLabel}</DialogTitle>
                        <DialogDescription>
                            Real chain action: {pendingAction?.chainActionType}
                        </DialogDescription>
                    </DialogHeader>

                    {pendingAction && (
                        <div className="space-y-4">
                            <div className="rounded-xl border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-sm text-amber-200">
                                <div className="font-semibold text-amber-100">Highest Risk Point</div>
                                <div className="mt-1 leading-relaxed">{pendingAction.highestRiskPoint}</div>
                            </div>

                            {pendingAction.kind === 'swap-execute' && (
                                <div className="rounded-xl border border-slate-700/80 bg-slate-950/40 px-4 py-3 text-sm">
                                    <div className="font-semibold text-slate-100">Frozen Payload Snapshot</div>
                                    <div className="mt-2 space-y-1 text-slate-300">
                                        <div>Calldata preview: {pendingAction.preparedTransactionSnapshot.calldataPreview}</div>
                                        <div>Calldata length: {pendingAction.preparedTransactionSnapshot.calldataLength}</div>
                                    </div>
                                </div>
                            )}

                            <div className="space-y-2 rounded-xl border px-4 py-3">
                                {pendingAction.confirmationFields.map((field) => (
                                    <div key={`${pendingAction.kind}-${field.label}`} className="flex items-start justify-between gap-4 text-sm">
                                        <span className="min-w-0 text-muted-foreground">{field.label}</span>
                                        <span className="max-w-[60%] break-all text-right font-medium text-foreground">{field.value}</span>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}

                    <DialogFooter>
                        <Button variant="outline" onClick={() => setPendingAction(null)} disabled={isSubmittingAction}>
                            Cancel
                        </Button>
                        <Button onClick={handleConfirmAction} disabled={isSubmittingAction}>
                            {isSubmittingAction ? 'Submitting...' : `Confirm ${pendingAction?.uiActionLabel ?? 'Action'}`}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </>
    );
};
