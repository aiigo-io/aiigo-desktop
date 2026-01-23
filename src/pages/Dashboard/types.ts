export interface DashboardStats {
    total_balance_usd: string;
    total_balance_btc: string;
    change_24h_amount: string;
    change_24h_percentage: string;
}

export interface PortfolioHistoryPoint {
    date: string;
    value: number;
}

export interface AssetAllocation {
    name: string;
    symbol: string;
    percentage: number;
    value_usd: number;
    color: string;
}


export interface BitcoinTransaction {
    id: string;
    wallet_id: string;
    tx_hash: string;
    tx_type: 'send' | 'receive';
    from_address: string;
    to_address: string;
    amount: number;
    fee: number;
    status: 'pending' | 'confirmed' | 'failed';
    confirmations: number;
    block_height: number | null;
    timestamp: string;
    created_at: string;
}

export interface EvmTransaction {
    id: string;
    wallet_id: string;
    tx_hash: string;
    tx_type: 'send' | 'receive' | 'approve' | 'contract';
    from_address: string;
    to_address: string;
    amount: string;
    amount_float: number;
    asset_symbol: string;
    asset_name: string;
    contract_address: string | null;
    chain: string;
    chain_id: number;
    gas_used: string;
    gas_price: string;
    fee: number;
    status: 'pending' | 'confirmed' | 'failed';
    block_number: number | null;
    timestamp: string;
    created_at: string;
}

export type UnifiedTransaction = {
    id: string;
    type: 'bitcoin' | 'evm';
    tx_type: 'send' | 'receive' | 'approve' | 'contract';
    tx_hash: string;
    asset_symbol: string;
    amount: string;
    timestamp: string;
}

export interface ChartDataPoint {
    day: string;
    value: number;
}
