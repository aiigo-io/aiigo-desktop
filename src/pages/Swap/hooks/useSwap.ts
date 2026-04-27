import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { Chain, Token, SUPPORTED_CHAINS, SUPPORTED_TOKENS, DEFAULT_SLIPPAGE, CHAIN_ID_TO_NAME } from '../constants';
import { openOceanService } from '../services/openocean.service';
import {
    QuoteResponse,
    RawEvmTransactionPayload,
    SwapApproveActionIntent,
    SwapExecuteActionIntent,
    SwapExecutionPayloadSnapshot,
    TransactionStatus,
} from '../types';
import { invoke, isTauriRuntimeAvailable, TAURI_UNAVAILABLE_MESSAGE } from '@/lib/tauri';

interface WalletInfo {
    id: string;
    address: string;
}

const NATIVE_TOKEN_ADDRESS = '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
const UNLIMITED_APPROVAL_AMOUNT = '0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';

const buildFingerprint = (parts: Array<string | number | undefined | null>): string => (
    parts.map((part) => String(part ?? '')).join('|')
);

const buildPayloadFingerprint = (payload: RawEvmTransactionPayload): string => (
    buildFingerprint([
        payload.to,
        payload.data,
        payload.value,
        payload.gasLimit,
        payload.gasPrice,
    ])
);

const buildCalldataPreview = (data: string): string => {
    if (data.length <= 18) {
        return data;
    }

    return `${data.slice(0, 10)}...${data.slice(-8)}`;
};

const buildPayloadSnapshot = (payload: RawEvmTransactionPayload): SwapExecutionPayloadSnapshot => ({
    to: payload.to,
    data: payload.data,
    value: payload.value,
    gasLimit: payload.gasLimit,
    gasPrice: payload.gasPrice,
    payloadFingerprint: buildPayloadFingerprint(payload),
    calldataPreview: buildCalldataPreview(payload.data),
    calldataLength: payload.data.length,
});

const payloadSnapshotMatches = (
    left: SwapExecutionPayloadSnapshot,
    right: SwapExecutionPayloadSnapshot,
): boolean => (
    left.to === right.to
    && left.data === right.data
    && left.value === right.value
    && left.gasLimit === right.gasLimit
    && left.gasPrice === right.gasPrice
    && left.payloadFingerprint === right.payloadFingerprint
);

const isUnlimitedApproval = (amount: string): boolean => (
    amount.trim().toLowerCase() === UNLIMITED_APPROVAL_AMOUNT
);

const formatTokenUnits = (rawAmount: string, decimals: number): string => {
    const numericAmount = Number(rawAmount);
    if (!Number.isFinite(numericAmount)) {
        return rawAmount;
    }

    return (numericAmount / Math.pow(10, decimals)).toFixed(6);
};

interface PreparedSwapExecution {
    rawTransaction: RawEvmTransactionPayload;
    payloadSnapshot: SwapExecutionPayloadSnapshot;
    swapData: {
        to: string;
        value: string;
        outAmount: string;
        outDecimals: number;
    };
}

export const useSwap = (wallet?: WalletInfo | null) => {
    const [fromChain, setFromChain] = useState<Chain>(SUPPORTED_CHAINS[0]);
    const [toChain, setToChain] = useState<Chain>(SUPPORTED_CHAINS[0]);

    const [fromToken, setFromToken] = useState<Token>(SUPPORTED_TOKENS[fromChain.id][0]);
    const [toToken, setToToken] = useState<Token>(SUPPORTED_TOKENS[toChain.id][1] || SUPPORTED_TOKENS[toChain.id][0]);

    const [amount, setAmount] = useState<string>('');
    const [slippage, setSlippage] = useState<number>(DEFAULT_SLIPPAGE);

    // Quote state
    const [quote, setQuote] = useState<QuoteResponse['data'] | null>(null);
    const [isLoadingQuote, setIsLoadingQuote] = useState(false);
    const [quoteError, setQuoteError] = useState<string | null>(null);

    // Gas price state
    const [gasPrice, setGasPrice] = useState<string>('');

    // Approval state
    const [needsApproval, setNeedsApproval] = useState(false);
    const [isCheckingApproval, setIsCheckingApproval] = useState(false);
    const [spenderAddress, setSpenderAddress] = useState<string | null>(null);

    // Transaction state
    const [txStatus, setTxStatus] = useState<TransactionStatus>({ status: 'idle' });

    // Debounce timer
    const quoteTimerRef = useRef<NodeJS.Timeout | null>(null);

    const fromTokens = useMemo(() => SUPPORTED_TOKENS[fromChain.id] || [], [fromChain.id]);
    const toTokens = useMemo(() => SUPPORTED_TOKENS[toChain.id] || [], [toChain.id]);

    const handleChainChange = useCallback((chain: Chain) => {
        setFromChain(chain);
        setToChain(chain);
        const tokens = SUPPORTED_TOKENS[chain.id] || [];
        if (tokens.length > 0) {
            setFromToken(tokens[0]);
            setToToken(tokens[1] || tokens[0]);
        }
        setQuote(null);
        setNeedsApproval(false);
    }, []);

    const flipAssets = useCallback(() => {
        const tempToken = fromToken;
        setFromToken(toToken);
        setToToken(tempToken);
        setQuote(null);
        setNeedsApproval(false);
    }, [fromToken, toToken]);

    const isValid = useMemo(() => {
        if (!amount || parseFloat(amount) <= 0) return false;
        if (fromToken.address === toToken.address) return false;
        return true;
    }, [amount, fromToken.address, toToken.address]);

    // Calculate price impact
    const priceImpact = useMemo(() => {
        if (!quote || !quote.priceImpact) return 0;
        return quote.priceImpact;
    }, [quote]);

    // Fetch gas price
    const fetchGasPrice = useCallback(async () => {
        try {
            const gasPriceResponse = await openOceanService.getGasPrice(fromChain.id);
            // Use the helper method to extract the gas price string
            const gasPriceString = openOceanService.getGasPriceString(gasPriceResponse);
            console.log('Fetched gas price (GWEI):', gasPriceString, 'for chain:', fromChain.name);
            setGasPrice(gasPriceString);
            return gasPriceString;
        } catch (error) {
            console.error('Error fetching gas price:', error);
            // Fallback to chain-specific default values
            // L2s like Arbitrum, Optimism have much lower gas prices
            const fallbackGasPrice = [42161, 10].includes(fromChain.id) ? '0.1' : '30';
            console.log('Using fallback gas price (GWEI):', fallbackGasPrice, 'for chain:', fromChain.name);
            setGasPrice(fallbackGasPrice);
            return fallbackGasPrice;
        }
    }, [fromChain.id, fromChain.name]);

    // Fetch quote
    const fetchQuote = useCallback(async () => {
        if (!isValid || !gasPrice) return;

        setIsLoadingQuote(true);
        setQuoteError(null);

        try {
            const chainName = CHAIN_ID_TO_NAME[fromChain.id];
            const quoteResponse = await openOceanService.getQuote({
                chain: chainName,
                inTokenAddress: fromToken.address,
                outTokenAddress: toToken.address,
                amount: amount,
                gasPrice: gasPrice,
                slippage: slippage,
            });

            setQuote(quoteResponse.data);
        } catch (error) {
            console.error('Error fetching quote:', error);
            setQuoteError(error instanceof Error ? error.message : 'Failed to fetch quote');
            setQuote(null);
        } finally {
            setIsLoadingQuote(false);
        }
    }, [isValid, gasPrice, fromChain.id, fromToken.address, toToken.address, amount, slippage]);

    // Check if approval is needed
    const checkApproval = useCallback(async () => {
        if (!wallet || !isValid || !amount || !gasPrice) {
            setNeedsApproval(false);
            setSpenderAddress(null);
            return;
        }

        // Native tokens don't need approval
        if (fromToken.address.toLowerCase() === NATIVE_TOKEN_ADDRESS) {
            setNeedsApproval(false);
            setSpenderAddress(null);
            return;
        }

        setIsCheckingApproval(true);
        try {
            // First, get the spender address from a swap quote
            const spender = await openOceanService.getSpenderAddress(
                fromChain.id,
                wallet.address,
                fromToken.address,
                toToken.address,
                amount,
                gasPrice,
                slippage
            );
            setSpenderAddress(spender);

            // Then check the actual allowance
            const approvalResult = await openOceanService.needsApproval(
                fromChain.id,
                wallet.address,
                fromToken.address,
                amount,
                fromToken.decimals
            );
            
            setNeedsApproval(approvalResult.needsApproval);
            
            console.log('Approval check:', {
                needsApproval: approvalResult.needsApproval,
                spender,
                currentAllowance: approvalResult.currentAllowance,
                requiredAmount: amount,
            });
        } catch (error) {
            console.error('Error checking approval:', error);
            setNeedsApproval(false);
            setSpenderAddress(null);
        } finally {
            setIsCheckingApproval(false);
        }
    }, [wallet, isValid, amount, gasPrice, fromChain.id, fromToken.address, fromToken.decimals, toToken.address, slippage]);

    const currentApprovalFingerprint = useCallback(() => buildFingerprint([
        'swap-approve',
        wallet?.id,
        fromChain.id,
        fromToken.address.toLowerCase(),
        spenderAddress?.toLowerCase(),
        UNLIMITED_APPROVAL_AMOUNT,
    ]), [wallet?.id, fromChain.id, fromToken.address, spenderAddress]);

    const currentSwapExecutionFingerprint = useCallback(() => buildFingerprint([
        'swap-execute',
        wallet?.id,
        fromChain.id,
        fromToken.address.toLowerCase(),
        toToken.address.toLowerCase(),
        amount,
        slippage,
    ]), [wallet?.id, fromChain.id, fromToken.address, toToken.address, amount, slippage]);

    const prepareApproveAction = useCallback(async (): Promise<SwapApproveActionIntent> => {
        if (!isTauriRuntimeAvailable()) {
            throw new Error(TAURI_UNAVAILABLE_MESSAGE);
        }

        if (!wallet) {
            throw new Error('No wallet connected');
        }

        if (!spenderAddress) {
            throw new Error('Spender address not found. Please wait for approval check to complete.');
        }

        const approvalMode = isUnlimitedApproval(UNLIMITED_APPROVAL_AMOUNT) ? 'unlimited' : 'bounded';

        return {
            kind: 'swap-approve',
            uiActionLabel: 'Approve For Swap',
            chainActionType: 'ERC20 approve(spender, amount)',
            highestRiskPoint: approvalMode === 'unlimited'
                ? 'Unlimited approval lets the spender move this token until you revoke the allowance.'
                : 'Approval grants the spender limited token access for the requested swap path.',
            chainId: fromChain.id,
            chainName: fromChain.name,
            confirmationFields: [
                { label: 'From', value: wallet.address, payloadField: 'walletId' },
                { label: 'Chain', value: fromChain.name, payloadField: 'chainId' },
                { label: 'Token', value: `${fromToken.symbol} (${fromToken.address})`, payloadField: 'tokenAddress' },
                { label: 'Spender', value: spenderAddress, payloadField: 'spenderAddress' },
                { label: 'Approval Amount', value: approvalMode === 'unlimited' ? 'Unlimited' : amount, payloadField: 'amount' },
                { label: 'Approval Scope', value: approvalMode, payloadField: 'amount' },
            ],
            approvalMode,
            fingerprint: currentApprovalFingerprint(),
            execution: {
                command: 'evm_approve_token',
                args: {
                    walletId: wallet.id,
                    chainId: fromChain.id,
                    tokenAddress: fromToken.address,
                    spenderAddress,
                    amount: UNLIMITED_APPROVAL_AMOUNT,
                },
            },
        };
    }, [wallet, spenderAddress, fromChain.id, fromChain.name, fromToken.address, fromToken.symbol, amount, currentApprovalFingerprint]);

    const submitApproveAction = useCallback(async (intent: SwapApproveActionIntent) => {
        if (intent.fingerprint !== currentApprovalFingerprint()) {
            const error = new Error('Approval parameters changed. Review the approval again.');
            setTxStatus({ status: 'error', error: error.message });
            throw error;
        }

        setTxStatus({ status: 'approving' });

        try {
            const txHash = await invoke<string>(intent.execution.command, intent.execution.args);
            setTxStatus({ status: 'success', hash: txHash });

            const waitTime = [42161, 10].includes(intent.chainId) ? 3000 : 10000;
            
            await new Promise(resolve => setTimeout(resolve, waitTime));
            await checkApproval();
            
            setTxStatus({ status: 'idle' });

            return txHash;
        } catch (error) {
            console.error('Approval error:', error);
            const errorMessage = error instanceof Error ? error.message : 'Approval failed';
            setTxStatus({ status: 'error', error: errorMessage });
            throw error;
        }
    }, [checkApproval, currentApprovalFingerprint]);

    const prepareCurrentSwapExecution = useCallback(async (): Promise<PreparedSwapExecution> => {
        if (!isTauriRuntimeAvailable()) {
            throw new Error(TAURI_UNAVAILABLE_MESSAGE);
        }

        if (!wallet || !isValid) {
            throw new Error('Invalid swap parameters');
        }

        if (fromToken.address.toLowerCase() !== NATIVE_TOKEN_ADDRESS) {
            const approvalStatus = await openOceanService.needsApproval(
                fromChain.id,
                wallet.address,
                fromToken.address,
                amount,
                fromToken.decimals
            );
            
            if (approvalStatus.needsApproval) {
                throw new Error(
                    `Insufficient token allowance. Current: ${approvalStatus.currentAllowance}, ` +
                    `Required: ${amount}. Please approve the token first.`
                );
            }
        }

        const currentGasPrice = await fetchGasPrice();
        const chainName = CHAIN_ID_TO_NAME[fromChain.id];
        const swapQuoteResponse = await openOceanService.getSwapQuote({
            chain: chainName,
            inTokenAddress: fromToken.address,
            outTokenAddress: toToken.address,
            amount,
            gasPrice: currentGasPrice,
            slippage,
            account: wallet.address,
        });

        const swapData = swapQuoteResponse.data;
        if (swapData.chainId !== fromChain.id) {
            throw new Error(
                `Chain mismatch: wallet is on chain ${fromChain.id} but transaction is for chain ${swapData.chainId}`
            );
        }

        const estimatedGas = Math.floor(swapData.estimatedGas * 1.25);
        const gasPriceInWei = openOceanService.convertGweiToWei(currentGasPrice);
        const rawTransaction = {
            to: swapData.to,
            data: swapData.data,
            value: swapData.value,
            gasLimit: estimatedGas.toString(),
            gasPrice: gasPriceInWei,
        };

        return {
            rawTransaction,
            payloadSnapshot: buildPayloadSnapshot(rawTransaction),
            swapData: {
                to: swapData.to,
                value: swapData.value,
                outAmount: swapData.outAmount,
                outDecimals: swapData.outToken.decimals,
            },
        };
    }, [wallet, isValid, fromToken.address, fromToken.decimals, amount, slippage, fetchGasPrice, fromChain.id, toToken.address]);

    const prepareSwapExecutionAction = useCallback(async (): Promise<SwapExecuteActionIntent> => {
        if (!wallet) {
            throw new Error('No wallet connected');
        }

        const preparedExecution = await prepareCurrentSwapExecution();

        return {
            kind: 'swap-execute',
            uiActionLabel: 'Execute Swap',
            chainActionType: 'Routed EVM contract call',
            highestRiskPoint: 'This is a contract call to the router target, not a normal send.',
            chainId: fromChain.id,
            chainName: fromChain.name,
            confirmationFields: [
                { label: 'From', value: wallet.address, payloadField: 'walletId' },
                { label: 'Chain', value: fromChain.name, payloadField: 'chainId' },
                { label: 'Router Target', value: preparedExecution.swapData.to, payloadField: 'transaction.to' },
                { label: 'Input Token', value: `${fromToken.symbol} (${fromToken.address})`, payloadField: 'transaction.data' },
                { label: 'Input Amount', value: amount, payloadField: 'transaction.data' },
                { label: 'Output Token', value: `${toToken.symbol} (${toToken.address})`, payloadField: 'transaction.data' },
                { label: 'Quoted Output', value: formatTokenUnits(preparedExecution.swapData.outAmount, preparedExecution.swapData.outDecimals), payloadField: 'transaction.data' },
                { label: 'Slippage', value: `${slippage}%`, payloadField: 'transaction.data' },
                { label: 'Value', value: preparedExecution.swapData.value, payloadField: 'transaction.value' },
                { label: 'Gas Limit', value: preparedExecution.rawTransaction.gasLimit, payloadField: 'transaction.gasLimit' },
                { label: 'Gas Price (wei)', value: preparedExecution.rawTransaction.gasPrice, payloadField: 'transaction.gasPrice' },
                { label: 'Calldata Preview', value: preparedExecution.payloadSnapshot.calldataPreview, payloadField: 'transaction.data' },
            ],
            fingerprint: currentSwapExecutionFingerprint(),
            preparedTransactionSnapshot: preparedExecution.payloadSnapshot,
            execution: {
                command: 'evm_send_transaction',
                args: {
                    walletId: wallet.id,
                    chainId: fromChain.id,
                    transaction: preparedExecution.rawTransaction,
                },
            },
        };
    }, [wallet, fromChain.id, fromChain.name, fromToken.address, fromToken.symbol, toToken.address, toToken.symbol, amount, slippage, currentSwapExecutionFingerprint, prepareCurrentSwapExecution]);

    const submitSwapExecutionAction = useCallback(async (intent: SwapExecuteActionIntent) => {
        const currentExecution = await prepareCurrentSwapExecution();
        if (!payloadSnapshotMatches(intent.preparedTransactionSnapshot, currentExecution.payloadSnapshot)) {
            const error = new Error('Swap execution payload changed. Review the swap again.');
            setTxStatus({ status: 'error', error: error.message });
            throw error;
        }

        setTxStatus({ status: 'swapping' });

        try {
            const txHash = await invoke<string>(intent.execution.command, intent.execution.args);
            setTxStatus({ status: 'success', hash: txHash });

            setTimeout(() => {
                setAmount('');
                setQuote(null);
                setTxStatus({ status: 'idle' });
            }, 3000);

            return txHash;
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Swap failed';
            setTxStatus({ status: 'error', error: errorMessage });
            throw error;
        }
    }, [prepareCurrentSwapExecution]);

    // Debounced quote fetching
    useEffect(() => {
        if (quoteTimerRef.current) {
            clearTimeout(quoteTimerRef.current);
        }

        if (isValid && gasPrice) {
            quoteTimerRef.current = setTimeout(() => {
                fetchQuote();
            }, 500); // 500ms debounce
        } else {
            setQuote(null);
        }

        return () => {
            if (quoteTimerRef.current) {
                clearTimeout(quoteTimerRef.current);
            }
        };
    }, [isValid, gasPrice, fromToken.address, toToken.address, amount, slippage, fetchQuote]);

    // Fetch gas price on mount and when chain changes
    useEffect(() => {
        fetchGasPrice();
    }, [fetchGasPrice]);

    // Check approval when wallet, amount, or token changes
    useEffect(() => {
        if (wallet && isValid) {
            checkApproval();
        } else {
            setNeedsApproval(false);
        }
    }, [wallet, isValid, checkApproval]);

    return {
        // Chain & Token state
        fromChain,
        fromToken,
        toToken,
        fromTokens,
        toTokens,
        handleChainChange,
        setFromToken,
        setToToken,
        flipAssets,

        // Amount & Slippage
        amount,
        setAmount,
        slippage,
        setSlippage,

        // Quote state
        quote,
        isLoadingQuote,
        quoteError,
        priceImpact,

        // Approval state
        needsApproval,
        isCheckingApproval,
        prepareApproveAction,
        submitApproveAction,

        // Transaction state
        txStatus,
        prepareSwapExecutionAction,
        submitSwapExecutionAction,

        // Validation
        isValid,

        // Gas price
        gasPrice,
        fetchGasPrice,
    };
};
