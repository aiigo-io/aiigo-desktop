import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { Chain, Token, SUPPORTED_CHAINS, SUPPORTED_TOKENS, DEFAULT_SLIPPAGE, CHAIN_ID_TO_NAME } from '../constants';
import { openOceanService } from '../services/openocean.service';
import { QuoteResponse, TransactionStatus } from '../types';
import { invoke } from '@tauri-apps/api/core';

interface WalletInfo {
    id: string;
    address: string;
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
        if (fromToken.address.toLowerCase() === '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee') {
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

    // Approve token
    const approveToken = useCallback(async () => {
        if (!wallet) {
            throw new Error('No wallet connected');
        }

        if (!spenderAddress) {
            throw new Error('Spender address not found. Please wait for approval check to complete.');
        }

        setTxStatus({ status: 'approving' });

        try {
            console.log('Approving token:', {
                token: fromToken.symbol,
                tokenAddress: fromToken.address,
                spender: spenderAddress,
                chain: fromChain.name,
            });

            // Call Tauri backend to approve token with the correct spender
            const txHash = await invoke<string>('evm_approve_token', {
                walletId: wallet.id,
                chainId: fromChain.id,
                tokenAddress: fromToken.address,
                spenderAddress: spenderAddress,
                // Approve max amount (2^256 - 1)
                amount: '0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff',
            });

            console.log('Approval transaction sent:', txHash);
            setTxStatus({ status: 'success', hash: txHash });

            // Wait for the approval transaction to be mined
            // On Layer 2s like Arbitrum, this is usually very fast (1-2 seconds)
            // On mainnet, it might take longer
            const waitTime = [42161, 10].includes(fromChain.id) ? 3000 : 10000;
            
            await new Promise(resolve => setTimeout(resolve, waitTime));

            // Re-check approval status
            console.log('Re-checking approval status after transaction...');
            await checkApproval();
            
            setTxStatus({ status: 'idle' });

            return txHash;
        } catch (error) {
            console.error('Approval error:', error);
            const errorMessage = error instanceof Error ? error.message : 'Approval failed';
            setTxStatus({ status: 'error', error: errorMessage });
            throw error;
        }
    }, [wallet, fromChain.id, fromChain.name, fromToken.address, fromToken.symbol, spenderAddress, checkApproval]);

    // Execute swap
    const executeSwap = useCallback(async () => {
        if (!wallet || !isValid) {
            throw new Error('Invalid swap parameters');
        }

        // Re-check approval status before executing swap (for non-native tokens)
        if (fromToken.address.toLowerCase() !== '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee') {
            console.log('Verifying approval before swap...');
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
            console.log('Approval verified, proceeding with swap...');
        }

        setTxStatus({ status: 'swapping' });

        try {
            // Fetch fresh gas price
            const currentGasPrice = await fetchGasPrice();

            // Get swap quote with transaction data
            const chainName = CHAIN_ID_TO_NAME[fromChain.id];
            const swapQuoteResponse = await openOceanService.getSwapQuote({
                chain: chainName,
                inTokenAddress: fromToken.address,
                outTokenAddress: toToken.address,
                amount: amount,
                gasPrice: currentGasPrice,
                slippage: slippage,
                account: wallet.address,
            });

            const swapData = swapQuoteResponse.data;

            // Validate chainId matches
            if (swapData.chainId !== fromChain.id) {
                throw new Error(
                    `Chain mismatch: wallet is on chain ${fromChain.id} but transaction is for chain ${swapData.chainId}`
                );
            }

            // Estimate gas with 1.25x multiplier as recommended
            const estimatedGas = Math.floor(swapData.estimatedGas * 1.25);

            // Convert gas price from GWEI to wei
            // OpenOcean returns gasPrice in the response, but we need to ensure it's in wei
            // The gasPrice from swap_quote API is already in wei format as a string
            // However, we should use the current gas price we fetched earlier
            const gasPriceInWei = openOceanService.convertGweiToWei(currentGasPrice);

            console.log('=== Gas Price Conversion ===');
            console.log('Current gas price (GWEI):', currentGasPrice);
            console.log('Converted to wei:', gasPriceInWei);
            console.log('Gas limit:', estimatedGas);
            console.log('Estimated gas cost (wei):', BigInt(gasPriceInWei) * BigInt(estimatedGas));
            console.log('Estimated gas cost (ETH):', Number(BigInt(gasPriceInWei) * BigInt(estimatedGas)) / 1e18);

            console.log({
                walletId: wallet.id,
                chainId: fromChain.id,
                transaction: {
                    to: swapData.to,
                    data: swapData.data,
                    value: swapData.value,
                    gasLimit: estimatedGas.toString(),
                    gasPrice: gasPriceInWei, // Use converted gas price in wei
                },
            })

            // Send transaction via Tauri backend
            const txHash = await invoke<string>('evm_send_transaction', {
                walletId: wallet.id,
                chainId: fromChain.id,
                transaction: {
                    to: swapData.to,
                    data: swapData.data,
                    value: swapData.value,
                    gasLimit: estimatedGas.toString(),
                    gasPrice: gasPriceInWei, // Use converted gas price in wei
                },
            });

            setTxStatus({ status: 'success', hash: txHash });

            // Reset form after successful swap
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
    }, [wallet, isValid, fetchGasPrice, fromChain.id, fromToken.address, toToken.address, amount, slippage]);

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
        approveToken,

        // Transaction state
        txStatus,
        executeSwap,

        // Validation
        isValid,

        // Gas price
        gasPrice,
        fetchGasPrice,
    };
};
