export type WalletType = 'mnemonic' | 'private-key';

export type FreshnessStatus = 'fresh' | 'cached' | 'stale' | 'unavailable' | 'partial';
export type ValuationStatus = 'valued' | 'unpriced';

export interface FreshnessMetadata {
  status: FreshnessStatus;
  updated_at: number | null;
  failed_sources: string[];
}

export interface SyncOutcome {
  reason: string;
  target: string;
  updated_at: number | null;
  partial: boolean;
  failed_sources: string[];
}

export interface EvmAsset {
  symbol: string;
  name: string;
  decimals: number;
  contract_address: string | null;
}

export interface EvmAssetBalance {
  chain: string;
  asset: EvmAsset;
  balance: string;
  balance_float: number;
  usd_price: number | null;
  usd_value: number | null;
  valuation_status: ValuationStatus;
}

export interface EvmChainAssets {
  chain: string;
  chain_id: number;
  total_balance_usd: number;
  valuation_status: ValuationStatus;
  unpriced_asset_count: number;
  freshness: FreshnessMetadata;
  assets: EvmAssetBalance[];
}

export interface EvmWalletInfo {
  id: string;
  label: string;
  wallet_type: WalletType;
  address: string;
  chains: EvmChainAssets[];
  total_balance_usd: number;
  valuation_status: ValuationStatus;
  unpriced_asset_count: number;
  created_at: string;
  updated_at: string;
}

export interface EvmWalletBalancesResponse {
  wallet: EvmWalletInfo;
  sync: SyncOutcome;
}

export interface SupportedEvmHistoryChain {
  chain: string;
  chain_id: number;
  display_name: string;
}

const SEPOLIA_CHAIN_ID = 11155111;

export function getWalletMainnetBalance(response: EvmWalletBalancesResponse): number {
  return response.wallet.chains
    .filter((chain) => chain.chain_id !== SEPOLIA_CHAIN_ID)
    .reduce((sum, chain) => sum + chain.total_balance_usd, 0);
}

export function getEvmChainAssets(
  response: EvmWalletBalancesResponse | null | undefined,
  chainId: number
): EvmChainAssets | undefined {
  return response?.wallet.chains.find((chain) => chain.chain_id === chainId);
}

export function formatFreshnessLabel(status: FreshnessStatus): string {
  switch (status) {
    case 'fresh':
      return 'Fresh';
    case 'cached':
      return 'Cached';
    case 'stale':
      return 'Stale';
    case 'unavailable':
      return 'Unavailable';
    case 'partial':
      return 'Partial';
  }
}

export function getFreshnessBadgeClass(status: FreshnessStatus): string {
  switch (status) {
    case 'fresh':
      return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600';
    case 'cached':
      return 'border-slate-500/30 bg-slate-500/10 text-slate-600';
    case 'stale':
      return 'border-amber-500/30 bg-amber-500/10 text-amber-700';
    case 'unavailable':
      return 'border-red-500/30 bg-red-500/10 text-red-700';
    case 'partial':
      return 'border-sky-500/30 bg-sky-500/10 text-sky-700';
  }
}

export function formatUnixTimestamp(updatedAt: number | null | undefined): string | null {
  if (!updatedAt) {
    return null;
  }

  return new Date(updatedAt * 1000).toLocaleTimeString();
}

export function getChainFreshnessDescription(freshness: FreshnessMetadata): string {
  const updatedAt = formatUnixTimestamp(freshness.updated_at);
  const failedSources = freshness.failed_sources.join(', ');
  const failedSourcesSuffix = failedSources ? ` Failed sources: ${failedSources}.` : '';

  switch (freshness.status) {
    case 'fresh':
      return updatedAt ? `On-chain data confirmed at ${updatedAt}.` : 'On-chain data is current.';
    case 'cached':
      return updatedAt
        ? `Showing cached balances from ${updatedAt}.${failedSourcesSuffix}`
        : 'Showing cached balances while refresh metadata is unavailable.';
    case 'stale':
      return updatedAt
        ? `Latest refresh failed. Showing cached balances from ${updatedAt}.${failedSourcesSuffix}`
        : `Latest refresh failed. Showing the last cached balances.${failedSourcesSuffix}`;
    case 'unavailable':
      return `This chain is unavailable and no cached balances could be loaded.${failedSourcesSuffix}`;
    case 'partial':
      return `This view is only partially up to date.${failedSourcesSuffix}`;
  }
}

export function getWalletSyncBanner(response: EvmWalletBalancesResponse): {
  label: string;
  description: string;
  className: string;
} | null {
  if (!response.sync.partial) {
    return null;
  }

  const failedSources = response.sync.failed_sources.join(', ');
  const updatedAt = formatUnixTimestamp(response.sync.updated_at);

  return {
    label: 'Partial Sync',
    description: updatedAt
      ? `Some chains failed to refresh at ${updatedAt}: ${failedSources || 'unknown source'}.`
      : `Some chains failed to refresh: ${failedSources || 'unknown source'}.`,
    className: 'border-amber-500/30 bg-amber-500/10 text-amber-800',
  };
}

export function getWalletUpdatedLabel(response: EvmWalletBalancesResponse | undefined): string {
  if (!response) {
    return 'Loading...';
  }

  const updatedAt = formatUnixTimestamp(response.sync.updated_at);
  if (!updatedAt) {
    return response.sync.partial ? 'Partial sync' : 'Status unavailable';
  }

  return response.sync.partial ? `Partial sync · ${updatedAt}` : `Updated: ${updatedAt}`;
}

export function getValuationStatusDescription(status: ValuationStatus, unpricedAssetCount: number): string | null {
  if (status !== 'unpriced' || unpricedAssetCount <= 0) {
    return null;
  }

  return `Priced subtotal only. ${unpricedAssetCount} unpriced asset${unpricedAssetCount === 1 ? '' : 's'} excluded.`;
}