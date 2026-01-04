import {
    QuoteParams,
    QuoteResponse,
    SwapQuoteParams,
    SwapQuoteResponse,
    AllowanceParams,
    AllowanceResponse,
    GasPriceResponse,
} from '../types';
import { OPENOCEAN_API_BASE, CHAIN_ID_TO_NAME } from '../constants';

class OpenOceanService {
    private baseUrl = OPENOCEAN_API_BASE;

    /**
     * Get chain name from chain ID
     */
    private getChainName(chainId: number): string {
        const chainName = CHAIN_ID_TO_NAME[chainId];
        if (!chainName) {
            throw new Error(`Unsupported chain ID: ${chainId}`);
        }
        return chainName;
    }

    /**
     * Build URL with query parameters
     */
    private buildUrl(path: string, params: Record<string, any>): string {
        const url = new URL(`${this.baseUrl}${path}`);
        Object.entries(params).forEach(([key, value]) => {
            if (value !== undefined && value !== null) {
                url.searchParams.append(key, String(value));
            }
        });
        return url.toString();
    }

    /**
     * Fetch quote for a token swap (without transaction data)
     */
    async getQuote(params: QuoteParams): Promise<QuoteResponse> {
        const url = this.buildUrl(`/v3/${params.chain}/quote`, {
            inTokenAddress: params.inTokenAddress,
            outTokenAddress: params.outTokenAddress,
            amount: params.amount,
            gasPrice: params.gasPrice,
            slippage: params.slippage || 1,
            disabledDexIds: params.disabledDexIds,
            enabledDexIds: params.enabledDexIds,
        });

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Quote API error: ${response.statusText}`);
        }

        const data: QuoteResponse = await response.json();
        if (data.code !== 200) {
            throw new Error(`Quote API error: ${JSON.stringify(data)}`);
        }

        return data;
    }

    /**
     * Fetch swap quote with transaction data
     */
    async getSwapQuote(params: SwapQuoteParams): Promise<SwapQuoteResponse> {
        const url = this.buildUrl(`/v3/${params.chain}/swap_quote`, {
            inTokenAddress: params.inTokenAddress,
            outTokenAddress: params.outTokenAddress,
            amount: params.amount,
            gasPrice: params.gasPrice,
            slippage: params.slippage || 1,
            account: params.account,
            referrer: params.referrer,
            referrerFee: params.referrerFee,
            sender: params.sender,
            minOutput: params.minOutput,
            disabledDexIds: params.disabledDexIds,
            enabledDexIds: params.enabledDexIds,
        });

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Swap Quote API error: ${response.statusText}`);
        }

        const data: SwapQuoteResponse = await response.json();
        if (data.code !== 200) {
            throw new Error(`Swap Quote API error: ${JSON.stringify(data)}`);
        }

        return data;
    }

    /**
     * Check token allowance
     */
    async getAllowance(params: AllowanceParams): Promise<AllowanceResponse> {
        const url = this.buildUrl(`/v3/${params.chain}/allowance`, {
            account: params.account,
            inTokenAddress: params.inTokenAddress,
        });

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Allowance API error: ${response.statusText}`);
        }

        const data: AllowanceResponse = await response.json();
        if (data.code !== 200) {
            throw new Error(`Allowance API error: ${JSON.stringify(data)}`);
        }

        return data;
    }

    /**
     * Get current gas price
     */
    async getGasPrice(chainId: number): Promise<GasPriceResponse> {
        const chainName = this.getChainName(chainId);
        const url = `${this.baseUrl}/v3/${chainName}/gasPrice`;

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Gas Price API error: ${response.statusText}`);
        }

        const data: GasPriceResponse = await response.json();
        if (data.code !== 200) {
            throw new Error(`Gas Price API error: ${JSON.stringify(data)}`);
        }

        return data;
    }

    /**
     * Get gas price as string for API calls (in GWEI without decimals)
     * Uses the 'fast' tier from without_decimals
     */
    getGasPriceString(gasPriceResponse: GasPriceResponse): string {
        // Use without_decimals.fast.legacyGasPrice as recommended
        return gasPriceResponse.without_decimals.fast.legacyGasPrice.toString();
    }

    /**
     * Check if approval is needed for a token
     * Returns { needsApproval: boolean, spender: string | null, currentAllowance: string }
     */
    async needsApproval(
        chainId: number,
        account: string,
        tokenAddress: string,
        amount: string,
        decimals: number
    ): Promise<{ needsApproval: boolean; spender: string | null; currentAllowance: string }> {
        // Native tokens don't need approval
        if (tokenAddress.toLowerCase() === '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee') {
            return { needsApproval: false, spender: null, currentAllowance: '0' };
        }

        const chainName = this.getChainName(chainId);
        const allowanceResponse = await this.getAllowance({
            chain: chainName,
            account,
            inTokenAddress: tokenAddress,
        });

        // OpenOcean API returns an array of allowances
        // We need to get the first item (or find the matching one)
        const allowanceData = allowanceResponse.data[0];
        if (!allowanceData) {
            // No allowance data found, assume needs approval
            return {
                needsApproval: true,
                spender: null,
                currentAllowance: '0',
            };
        }

        // Convert amount to wei/smallest unit
        const amountInWei = BigInt(Math.floor(parseFloat(amount) * Math.pow(10, decimals)));
        const allowance = BigInt(allowanceData.allowance);
        const needsApproval = allowance < amountInWei;

        console.log('Allowance check:', {
            token: tokenAddress,
            currentAllowance: allowanceData.allowance,
            requiredAmount: amountInWei.toString(),
            needsApproval,
        });

        return {
            needsApproval,
            spender: null, // OpenOcean doesn't return spender in allowance API
            currentAllowance: allowanceData.allowance,
        };
    }

    /**
     * Get spender address for approval (OpenOcean router contract)
     * This fetches a swap quote to get the correct spender address
     */
    async getSpenderAddress(
        chainId: number,
        account: string,
        inTokenAddress: string,
        outTokenAddress: string,
        amount: string,
        gasPrice: string,
        slippage: number
    ): Promise<string> {
        const chainName = this.getChainName(chainId);

        // Get swap quote to extract the 'to' address (spender)
        const swapQuote = await this.getSwapQuote({
            chain: chainName,
            inTokenAddress,
            outTokenAddress,
            amount,
            gasPrice,
            slippage,
            account,
        });

        return swapQuote.data.to;
    }

    /**
     * Convert gas price from GWEI to wei
     * OpenOcean API returns gas price in GWEI (e.g., "0.074530561")
     * We need to convert to wei for transactions
     */
    convertGweiToWei(gweiString: string): string {
        const gweiValue = parseFloat(gweiString);
        // 1 GWEI = 1e9 wei
        const weiValue = Math.floor(gweiValue * 1e9);
        return weiValue.toString();
    }
}

export const openOceanService = new OpenOceanService();
