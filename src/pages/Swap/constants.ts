export interface Token {
    symbol: string;
    name: string;
    decimals: number;
    address: string;
    logoURI: string;
}

export interface Chain {
    id: number;
    name: string;
    symbol: string;
    logoURI: string;
    nativeCurrency: {
        name: string;
        symbol: string;
        decimals: number;
    };
}

export const SUPPORTED_CHAINS: Chain[] = [
    {
        id: 1,
        name: 'Ethereum',
        symbol: 'ETH',
        logoURI: 'https://cryptologos.cc/logos/ethereum-eth-logo.svg',
        nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    },
    {
        id: 42161,
        name: 'Arbitrum',
        symbol: 'ARB',
        logoURI: 'https://cryptologos.cc/logos/arbitrum-arb-logo.svg',
        nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    },
    {
        id: 10,
        name: 'Optimism',
        symbol: 'OP',
        logoURI: 'https://cryptologos.cc/logos/optimism-ethereum-op-logo.svg',
        nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    },
    {
        id: 137,
        name: 'Polygon',
        symbol: 'MATIC',
        logoURI: 'https://cryptologos.cc/logos/polygon-matic-logo.svg',
        nativeCurrency: { name: 'MATIC', symbol: 'MATIC', decimals: 18 },
    },
    {
        id: 56,
        name: 'BSC',
        symbol: 'BNB',
        logoURI: 'https://cryptologos.cc/logos/bnb-bnb-logo.svg',
        nativeCurrency: { name: 'BNB', symbol: 'BNB', decimals: 18 },
    },
];

export const SUPPORTED_TOKENS: Record<number, Token[]> = {
    1: [
        { symbol: 'ETH', name: 'Ethereum', decimals: 18, address: '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', logoURI: 'https://cryptologos.cc/logos/ethereum-eth-logo.svg' },
        { symbol: 'USDT', name: 'Tether USD', decimals: 6, address: '0xdac17f958d2ee523a2206206994597c13d831ec7', logoURI: 'https://cryptologos.cc/logos/tether-usdt-logo.svg' },
        { symbol: 'USDC', name: 'USD Coin', decimals: 6, address: '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48', logoURI: 'https://cryptologos.cc/logos/usd-coin-usdc-logo.svg' },
    ],
    42161: [
        { symbol: 'ETH', name: 'Ethereum', decimals: 18, address: '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', logoURI: 'https://cryptologos.cc/logos/ethereum-eth-logo.svg' },
        { symbol: 'USDT', name: 'Tether USD', decimals: 6, address: '0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9', logoURI: 'https://cryptologos.cc/logos/tether-usdt-logo.svg' },
        { symbol: 'USDC', name: 'USD Coin', decimals: 6, address: '0xaf88d065e77c8cC2239327C5EDb3A432268e5831', logoURI: 'https://cryptologos.cc/logos/usd-coin-usdc-logo.svg' },
    ],
    10: [
        { symbol: 'ETH', name: 'Ethereum', decimals: 18, address: '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', logoURI: 'https://cryptologos.cc/logos/ethereum-eth-logo.svg' },
        { symbol: 'USDT', name: 'Tether USD', decimals: 6, address: '0x94b008aa00579c1307b0ef2c499ad98a8ce58e58', logoURI: 'https://cryptologos.cc/logos/tether-usdt-logo.svg' },
        { symbol: 'USDC', name: 'USD Coin', decimals: 6, address: '0x0b2c639c533813f4aa9d7837caf62653d097ff85', logoURI: 'https://cryptologos.cc/logos/usd-coin-usdc-logo.svg' },
    ],
    137: [
        { symbol: 'MATIC', name: 'Polygon', decimals: 18, address: '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', logoURI: 'https://cryptologos.cc/logos/polygon-matic-logo.svg' },
        { symbol: 'USDT', name: 'Tether USD', decimals: 6, address: '0xc2132d05d31c914a87c6611c10748aeb04b58e8f', logoURI: 'https://cryptologos.cc/logos/tether-usdt-logo.svg' },
        { symbol: 'USDC', name: 'USD Coin', decimals: 6, address: '0x3c499c542cef5e3811e1192ce70d8cc03d5c3359', logoURI: 'https://cryptologos.cc/logos/usd-coin-usdc-logo.svg' },
    ],
    56: [
        { symbol: 'BNB', name: 'Binance Coin', decimals: 18, address: '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', logoURI: 'https://cryptologos.cc/logos/bnb-bnb-logo.svg' },
        { symbol: 'USDT', name: 'Tether USD', decimals: 18, address: '0x55d398326f99059fF775485246999027B3197955', logoURI: 'https://cryptologos.cc/logos/tether-usdt-logo.svg' },
        { symbol: 'USDC', name: 'USD Coin', decimals: 18, address: '0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d', logoURI: 'https://cryptologos.cc/logos/usd-coin-usdc-logo.svg' },
    ],
};

// OpenOcean API Configuration
export const OPENOCEAN_API_BASE = 'https://open-api.openocean.finance';

// Chain ID to OpenOcean chain name mapping
export const CHAIN_ID_TO_NAME: Record<number, string> = {
    1: 'eth',
    42161: 'arbitrum',
    10: 'optimism',
    137: 'polygon',
    56: 'bsc',
};

// Swap Configuration
export const DEFAULT_SLIPPAGE = 1; // 1%
export const MAX_PRICE_IMPACT_WARNING = 5; // 5%
export const MAX_PRICE_IMPACT_BLOCK = 15; // 15%
export const QUOTE_REFRESH_INTERVAL = 10000; // 10 seconds

// Native token address used by OpenOcean
export const NATIVE_TOKEN_ADDRESS = '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
