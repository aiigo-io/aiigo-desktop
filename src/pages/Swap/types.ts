// OpenOcean API Type Definitions

export interface QuoteParams {
    chain: string;
    inTokenAddress: string;
    outTokenAddress: string;
    amount: string;
    gasPrice: string;
    slippage?: number;
    disabledDexIds?: string;
    enabledDexIds?: string;
}

export interface QuoteResponse {
    code: number;
    data: {
        inToken: {
            symbol: string;
            name: string;
            address: string;
            decimals: number;
        };
        outToken: {
            symbol: string;
            name: string;
            address: string;
            decimals: number;
        };
        inAmount: string;
        outAmount: string;
        estimatedGas: number;
        path?: {
            dexName: string;
            percentage: number;
        }[];
        priceImpact?: number;
    };
}

export interface SwapQuoteParams extends QuoteParams {
    account: string;
    referrer?: string;
    referrerFee?: number;
    sender?: string;
    minOutput?: number;
}

export interface SwapQuoteResponse {
    code: number;
    data: {
        inToken: {
            symbol: string;
            name: string;
            address: string;
            decimals: number;
        };
        outToken: {
            symbol: string;
            name: string;
            address: string;
            decimals: number;
        };
        inAmount: string;
        outAmount: string;
        estimatedGas: number;
        minOutAmount: string;
        path?: {
            dexName: string;
            percentage: number;
        }[];
        priceImpact?: number;
        // Transaction data
        from: string;
        to: string;
        data: string;
        value: string;
        gasPrice: string;
        chainId: number;
    };
}

export interface AllowanceParams {
    chain: string;
    account: string;
    inTokenAddress: string;
}

export interface AllowanceResponse {
    code: number;
    data: Array<{
        symbol: string;
        allowance: string;
        raw: string;
    }>;
}


export interface GasPriceResponse {
    code: number;
    data: {
        base: number;
        standard: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
        fast: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
        instant: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
        low: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
    };
    without_decimals: {
        base: number;
        standard: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
        fast: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
        instant: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
        low: {
            legacyGasPrice: number;
            maxPriorityFeePerGas: number;
            maxFeePerGas: number;
            waitTimeEstimate: number;
        };
    };
}

export interface SwapState {
    quote: QuoteResponse['data'] | null;
    swapQuote: SwapQuoteResponse['data'] | null;
    isLoadingQuote: boolean;
    priceImpact: number;
    slippage: number;
    needsApproval: boolean;
    gasPrice: string;
    error: string | null;
}

export interface TransactionStatus {
    status: 'idle' | 'approving' | 'swapping' | 'success' | 'error';
    hash?: string;
    error?: string;
}

export type ApprovalMode = 'bounded' | 'unlimited';

export interface ActionConfirmationField {
    label: string;
    value: string;
    payloadField?: string;
}

export interface RawEvmTransactionPayload {
    to: string;
    data: string;
    value: string;
    gasLimit: string;
    gasPrice: string;
}

export interface SwapExecutionPayloadSnapshot {
    to: string;
    data: string;
    value: string;
    gasLimit: string;
    gasPrice: string;
    payloadFingerprint: string;
    calldataPreview: string;
    calldataLength: number;
}

interface BaseSwapActionIntent {
    kind: 'swap-approve' | 'swap-execute';
    uiActionLabel: string;
    chainActionType: string;
    highestRiskPoint: string;
    chainId: number;
    chainName: string;
    confirmationFields: ActionConfirmationField[];
    fingerprint: string;
}

export interface SwapApproveActionIntent extends BaseSwapActionIntent {
    kind: 'swap-approve';
    approvalMode: ApprovalMode;
    execution: {
        command: 'evm_approve_token';
        args: {
            walletId: string;
            chainId: number;
            tokenAddress: string;
            spenderAddress: string;
            amount: string;
        };
    };
}

export interface SwapExecuteActionIntent extends BaseSwapActionIntent {
    preparedTransactionSnapshot: SwapExecutionPayloadSnapshot;
    kind: 'swap-execute';
    execution: {
        command: 'evm_send_transaction';
        args: {
            walletId: string;
            chainId: number;
            transaction: RawEvmTransactionPayload;
        };
    };
}

export type SwapActionIntent = SwapApproveActionIntent | SwapExecuteActionIntent;
