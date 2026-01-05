import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/card';
import { ArrowUpRight, ArrowDownLeft, Activity, Wallet, TrendingUp } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { shortAddress } from '@/lib/utils';

interface DashboardStats {
  total_balance_usd: string;
  total_balance_btc: string;
  change_24h_amount: string;
  change_24h_percentage: string;
}

interface PortfolioHistoryPoint {
  date: string;
  value: number;
}

interface AssetAllocation {
  name: string;
  symbol: string;
  percentage: number;
  value_usd: number;
  color: string;
}

interface TopMover {
  symbol: string;
  name: string;
  price_usd: number;
  change_24h: number;
  is_positive: boolean;
}

interface BitcoinTransaction {
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

interface EvmTransaction {
  id: string;
  wallet_id: string;
  tx_hash: string;
  tx_type: 'send' | 'receive';
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

type UnifiedTransaction = {
  id: string;
  type: 'bitcoin' | 'evm';
  tx_type: 'send' | 'receive';
  tx_hash: string;
  asset_symbol: string;
  amount: string;
  timestamp: string;
}

const Dashboard: React.FC = () => {
  const navigate = useNavigate();

  // Stats state
  const [stats, setStats] = useState<DashboardStats>({
    total_balance_usd: '$0.00',
    total_balance_btc: '≈ 0.00 BTC',
    change_24h_amount: '+$0.00',
    change_24h_percentage: '+0.00%'
  });

  // Chart data state
  const [chartData, setChartData] = useState<Array<{ day: string; value: number }>>([]);

  // Asset allocation state
  const [allocation, setAllocation] = useState<AssetAllocation[]>([]);

  // Top movers state
  const [topMovers, setTopMovers] = useState<TopMover[]>([]);

  // Transactions state
  const [recentTransactions, setRecentTransactions] = useState<UnifiedTransaction[]>([]);

  useEffect(() => {
    const loadData = async () => {
      try {
        // 1. Load cached data immediately
        const cachedStats = await invoke<DashboardStats>('get_dashboard_stats');
        setStats(cachedStats);

        // 2. Load chart history
        const history = await invoke<PortfolioHistoryPoint[]>('get_portfolio_history');
        if (history && history.length > 0) {
          const formattedData = history.map(point => {
            const date = new Date(point.date);
            const dayName = date.toLocaleDateString('en-US', { weekday: 'short' });
            return {
              day: dayName,
              value: point.value
            };
          });
          setChartData(formattedData);
        }

        // 3. Load asset allocation
        const allocationData = await invoke<AssetAllocation[]>('get_asset_allocation');
        setAllocation(allocationData);

        // 4. Load recent transactions
        await loadRecentTransactions();

        // 5. Refresh data in background
        const freshStats = await invoke<DashboardStats>('refresh_dashboard_stats');
        setStats(freshStats);

        // 6. Reload chart after refresh
        const updatedHistory = await invoke<PortfolioHistoryPoint[]>('get_portfolio_history');
        if (updatedHistory && updatedHistory.length > 0) {
          const formattedData = updatedHistory.map(point => {
            const date = new Date(point.date);
            const dayName = date.toLocaleDateString('en-US', { weekday: 'short' });
            return {
              day: dayName,
              value: point.value
            };
          });
          setChartData(formattedData);
        }

        // 7. Reload asset allocation after refresh
        const updatedAllocation = await invoke<AssetAllocation[]>('get_asset_allocation');
        setAllocation(updatedAllocation);

        // 8. Load top movers
        const moversData = await invoke<TopMover[]>('get_top_movers');
        setTopMovers(moversData);
      } catch (error) {
        console.error('Failed to load dashboard stats:', error);
      }
    };

    loadData();
  }, []);

  const loadRecentTransactions = async () => {
    try {
      const [btcTxs, evmTxs] = await Promise.all([
        invoke<BitcoinTransaction[]>('get_all_bitcoin_transactions'),
        invoke<EvmTransaction[]>('get_all_evm_transactions'),
      ]);

      const unifiedBtcTxs: UnifiedTransaction[] = btcTxs
        .filter(tx => tx.status !== 'failed')
        .map(tx => ({
          id: tx.id,
          type: 'bitcoin' as const,
          tx_type: tx.tx_type,
          tx_hash: tx.tx_hash,
          asset_symbol: 'BTC',
          amount: tx.amount.toFixed(8),
          timestamp: tx.timestamp,
        }));

      const unifiedEvmTxs: UnifiedTransaction[] = evmTxs
        .filter(tx => tx.status !== 'failed')
        .map(tx => ({
          id: tx.id,
          type: 'evm' as const,
          tx_type: tx.tx_type,
          tx_hash: tx.tx_hash,
          asset_symbol: tx.asset_symbol,
          amount: tx.amount_float.toFixed(6),
          timestamp: tx.timestamp,
        }));

      const allTxs = [...unifiedBtcTxs, ...unifiedEvmTxs].sort(
        (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
      );

      setRecentTransactions(allTxs.slice(0, 5));
    } catch (error) {
      console.error('Failed to load recent transactions:', error);
    }
  };

  const formatTimeAgo = (timestamp: string) => {
    const now = new Date();
    const txDate = new Date(timestamp);
    const diffMs = now.getTime() - txDate.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins} minute${diffMins > 1 ? 's' : ''} ago`;
    if (diffHours < 24) return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
    if (diffDays < 7) return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
    return txDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  };

  const maxValue = chartData.length > 0 ? Math.max(...chartData.map(d => d.value)) : 0;
  const minValue = chartData.length > 0 ? Math.min(...chartData.map(d => d.value)) : 0;

  const valueRange = maxValue - minValue;
  const baseValue = Math.max(maxValue, 1);
  const padding = valueRange > 0 ? valueRange * 0.1 : baseValue * 0.2;
  const chartMinValue = Math.max(0, minValue - padding);
  const chartMaxValue = maxValue + padding;

  const getDayLabels = () => {
    const labels = [];
    for (let i = 6; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      labels.push(date.toLocaleDateString('en-US', { weekday: 'short' }));
    }
    return labels;
  };
  const dayLabels = getDayLabels();

  return (
    <div className="min-h-screen p-6 space-y-6 font-sans bg-background selection:bg-primary/20">
      <div className="space-y-6 max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between py-2">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight text-foreground">
              Dashboard
            </h1>
            <p className="text-muted-foreground mt-1 flex items-center gap-2 text-xs font-mono">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]" />
              SYSTEM_ONLINE
            </p>
          </div>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Total Balance Card */}
          <Card className="p-6 glass-card relative overflow-hidden group">
            <div className="relative z-10">
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-[11px] font-bold text-muted-foreground uppercase tracking-widest font-mono">Total Balance</h3>
                <Wallet className="w-4 h-4 text-muted-foreground/50" />
              </div>
              <div className="space-y-1">
                <p className="text-4xl font-light tracking-tight text-foreground font-mono">
                  {stats.total_balance_usd}
                </p>
                <p className="text-xs font-mono text-muted-foreground/80 flex items-center gap-2">
                  <span className="text-primary">{stats.total_balance_btc}</span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded-sm bg-muted text-muted-foreground">BTC</span>
                </p>
              </div>
            </div>
          </Card>

          {/* 24h Change Card */}
          <Card className="p-6 glass-card relative overflow-hidden group">
            <div className="relative z-10">
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-[11px] font-bold text-muted-foreground uppercase tracking-widest font-mono">24h Performance</h3>
                <Activity className="w-4 h-4 text-muted-foreground/50" />
              </div>
              <div className="flex items-end justify-between">
                <div>
                  <p className={`text-4xl font-light tracking-tight font-mono ${stats.change_24h_amount.startsWith('+') ? 'text-emerald-400' : 'text-destructive'}`}>
                    {stats.change_24h_amount}
                  </p>
                  <p className="text-xs font-mono text-muted-foreground/80 mt-1">{stats.change_24h_percentage}</p>
                </div>
                {stats.change_24h_amount.startsWith('+') ? (
                  <ArrowUpRight className="w-8 h-8 text-emerald-500/20" />
                ) : (
                  <ArrowDownLeft className="w-8 h-8 text-destructive/20" />
                )}
              </div>
            </div>
          </Card>
        </div>

        {/* Portfolio Value Chart */}
        <Card className="p-6 glass-card">
          <div className="flex items-center justify-between mb-8">
            <div className="flex items-center gap-2">
              <TrendingUp className="w-4 h-4 text-muted-foreground" />
              <h3 className="text-sm font-semibold text-foreground">Portfolio Performance</h3>
            </div>
            <div className="flex bg-muted/50 p-0.5 rounded-lg border border-border/50">
              {['1H', '1D', '1W', '1M', '1Y', 'ALL'].map((period) => (
                <button
                  key={period}
                  className={`px-3 py-1 text-[10px] font-medium rounded-md transition-all ${period === '1W'
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground hover:bg-background/50'
                    }`}
                >
                  {period}
                </button>
              ))}
            </div>
          </div>

          <div className="h-64 px-2">
            <div className="flex h-full">
              <div className="flex-1 flex gap-2 items-end justify-center">
                {dayLabels.map((dayLabel, index) => {
                  const dataPoint = chartData.find(d => d.day === dayLabel);
                  const hasData = dataPoint !== undefined;
                  const value = hasData ? dataPoint.value : 0;

                  let height = 0;
                  if (hasData && chartMaxValue > chartMinValue) {
                    height = Math.max(10, ((value - chartMinValue) / (chartMaxValue - chartMinValue)) * 100);
                  } else if (hasData) {
                    height = 70;
                  }

                  return (
                    <div key={index} className="w-[13%] flex flex-col items-center gap-3 group h-full">
                      <div className="w-full relative h-full flex items-end justify-center">
                        {hasData ? (
                          <div
                            className="w-full bg-primary/20 border-t-2 border-primary rounded-sm transition-all duration-300 group-hover:bg-primary/30 relative"
                            style={{ height: `${height}%` }}
                          >
                            <div className="absolute -top-8 left-1/2 -translate-x-1/2 bg-popover text-popover-foreground border border-border text-[10px] py-1 px-2 rounded shadow-sm opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap z-20 font-mono">
                              ${value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                            </div>
                          </div>
                        ) : (
                          <div className="w-full h-0.5 bg-muted rounded-full" />
                        )}
                      </div>
                      <span className={`text-[10px] font-mono transition-colors ${hasData ? 'text-muted-foreground group-hover:text-foreground' : 'text-muted-foreground/30'}`}>
                        {dayLabel}
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </Card>

        {/* Asset Allocation and Top Movers */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Asset Allocation */}
          <Card className="p-6 glass-card">
            <h3 className="text-sm font-semibold text-foreground mb-6 flex items-center gap-2">
              Allocation
            </h3>
            {allocation.length > 0 ? (
              <div className="space-y-4">
                {allocation.map((asset, index) => (
                  <div key={index} className="space-y-2 group">
                    <div className="flex justify-between items-center text-sm">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-foreground">{asset.symbol}</span>
                        <span className="text-xs text-muted-foreground">{asset.name}</span>
                      </div>
                      <div className="text-right">
                        <span className="font-mono text-foreground">{asset.percentage.toFixed(1)}%</span>
                        <span className="text-xs text-muted-foreground ml-2 font-mono">
                          ${asset.value_usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </span>
                      </div>
                    </div>
                    <div className="w-full bg-muted/30 rounded-full h-1 overflow-hidden">
                      <div
                        className={`h-full ${asset.color} opacity-80 rounded-full transition-all duration-500`}
                        style={{ width: `${Math.max(asset.percentage, 1)}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground">
                <p className="text-xs">No assets found</p>
              </div>
            )}
          </Card>

          {/* Top Movers */}
          <Card className="p-6 glass-card">
            <h3 className="text-sm font-semibold text-foreground mb-6 flex items-center gap-2">
              Market Prices
            </h3>
            {topMovers.length > 0 ? (
              <div className="space-y-2">
                {topMovers.map((mover, index) => {
                  return (
                    <div
                      key={index}
                      className="flex items-center justify-between p-2 rounded-md hover:bg-muted/30 transition-all cursor-pointer group"
                    >
                      <div className="flex items-center gap-3">
                        <div className="w-6 h-6 rounded bg-muted/50 flex items-center justify-center text-[10px] font-bold text-foreground">
                          {mover.symbol[0]}
                        </div>
                        <div className="flex flex-col">
                          <span className="text-sm font-medium text-foreground leading-none">{mover.symbol}</span>
                          <span className="text-[10px] text-muted-foreground mt-0.5">
                            ${mover.price_usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                          </span>
                        </div>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className={`font-mono text-xs font-medium ${mover.is_positive ? 'text-emerald-400' : 'text-destructive'}`}>
                          {mover.is_positive ? '+' : ''}{mover.change_24h.toFixed(2)}%
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground">
                <p className="text-xs">Loading market data...</p>
              </div>
            )}
          </Card>
        </div>

        {/* Recent Transactions */}
        <Card className="p-6 glass-card">
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
              Recent Activity
            </h3>
            <button
              onClick={() => navigate('/transactions')}
              className="text-xs font-medium text-muted-foreground hover:text-foreground transition-colors"
            >
              View All
            </button>
          </div>

          <div className="space-y-1">
            {recentTransactions.length > 0 ? (
              <div className="relative overflow-x-auto">
                <table className="w-full text-xs text-left">
                  <thead className="text-[10px] text-muted-foreground uppercase bg-muted/20 font-mono">
                    <tr>
                      <th className="px-4 py-2 rounded-l-sm">Type</th>
                      <th className="px-4 py-2">Asset</th>
                      <th className="px-4 py-2">Hash</th>
                      <th className="px-4 py-2 text-right">Amount</th>
                      <th className="px-4 py-2 rounded-r-sm text-right">Time</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recentTransactions.map((tx) => {
                      const isSend = tx.tx_type === 'send';
                      return (
                        <tr key={tx.id} className="border-b border-border/50 hover:bg-muted/10 transition-colors">
                          <td className="px-4 py-3">
                            <span className={`inline-flex items-center gap-1.5 font-medium ${isSend ? 'text-destructive' : 'text-emerald-400'}`}>
                              {isSend ? (
                                <ArrowUpRight className="w-3 h-3" />
                              ) : (
                                <ArrowDownLeft className="w-3 h-3" />
                              )}
                              {isSend ? 'Send' : 'Receive'}
                            </span>
                          </td>
                          <td className="px-4 py-3 font-medium text-foreground">
                            {tx.asset_symbol}
                          </td>
                          <td className="px-4 py-3 font-mono text-muted-foreground">
                            {shortAddress(tx.tx_hash)}
                          </td>
                          <td className={`px-4 py-3 text-right font-mono ${isSend ? 'text-destructive' : 'text-emerald-400'}`}>
                            {isSend ? '-' : '+'}{tx.amount}
                          </td>
                          <td className="px-4 py-3 text-right text-muted-foreground">
                            {formatTimeAgo(tx.timestamp)}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground">
                <p className="text-xs">No recent transactions</p>
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
};

export default Dashboard;
