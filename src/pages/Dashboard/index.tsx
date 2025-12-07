import React, { useEffect, useState } from 'react';
import { Card } from '@/components/ui/card';
import { ArrowUpRight, ArrowDownLeft, Activity, Wallet, TrendingUp, Clock } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

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

const Dashboard: React.FC = () => {
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
            // Format date as "Mon", "Tue", etc.
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

        // 4. Refresh data in background
        const freshStats = await invoke<DashboardStats>('refresh_dashboard_stats');
        setStats(freshStats);

        // 5. Reload chart after refresh
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

        // 6. Reload asset allocation after refresh
        const updatedAllocation = await invoke<AssetAllocation[]>('get_asset_allocation');
        setAllocation(updatedAllocation);

        // 7. Load top movers (token price changes)
        const moversData = await invoke<TopMover[]>('get_top_movers');
        setTopMovers(moversData);
      } catch (error) {
        console.error('Failed to load dashboard stats:', error);
      }
    };

    loadData();
  }, []);

  const maxValue = chartData.length > 0 ? Math.max(...chartData.map(d => d.value)) : 0;
  const minValue = chartData.length > 0 ? Math.min(...chartData.map(d => d.value)) : 0;

  // Calculate a reasonable range for the chart
  // Handle edge cases: single data point or all values are the same
  const valueRange = maxValue - minValue;
  const baseValue = Math.max(maxValue, 1); // Prevent division by zero
  const padding = valueRange > 0 ? valueRange * 0.1 : baseValue * 0.2;
  const chartMinValue = Math.max(0, minValue - padding);
  const chartMaxValue = maxValue + padding;



  const recentTransactions = [
    { type: 'sent', asset: 'BTC', amount: '-0.05 BTC', time: '2 hours ago', hash: '0x3a...8f21' },
    { type: 'received', asset: 'ETH', amount: '+1.2 ETH', time: '5 hours ago', hash: '0x7b...9c44' }
  ];

  // Generate 7-day labels
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
    <div className="min-h-screen bg-slate-50 p-6 space-y-8 text-slate-900 font-sans selection:bg-cyan-200/50">
      {/* Background Effects */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-blue-200/20 rounded-full blur-[120px]" />
        <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-purple-200/20 rounded-full blur-[120px]" />
      </div>

      <div className="relative z-10 space-y-8 max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-4xl font-bold bg-gradient-to-r from-slate-900 to-slate-600 bg-clip-text text-transparent">
              Dashboard
            </h1>
            <p className="text-slate-500 mt-1 flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
              Mainnet Connected
            </p>
          </div>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Total Balance Card */}
          <Card className="p-6 bg-white/60 border-white/40 backdrop-blur-xl shadow-lg shadow-slate-200/50 relative overflow-hidden group hover:shadow-xl transition-all duration-300">
            <div className="absolute inset-0 bg-gradient-to-br from-blue-50/50 to-purple-50/50 opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
            <div className="relative z-10">
              <div className="flex items-center gap-3 mb-4">
                <div className="p-2 rounded-lg bg-blue-100 text-blue-600">
                  <Wallet className="w-5 h-5" />
                </div>
                <h3 className="text-sm font-medium text-slate-500 uppercase tracking-wider">Total Balance</h3>
              </div>
              <div className="space-y-1">
                <p className="text-4xl font-bold font-mono text-slate-900 tracking-tight">
                  {stats.total_balance_usd}
                </p>
                <p className="text-sm font-mono text-slate-500">{stats.total_balance_btc}</p>
              </div>
            </div>
          </Card>

          {/* 24h Change Card */}
          <Card className="p-6 bg-white/60 border-white/40 backdrop-blur-xl shadow-lg shadow-slate-200/50 relative overflow-hidden group hover:shadow-xl transition-all duration-300">
            <div className="absolute inset-0 bg-gradient-to-br from-emerald-50/50 to-teal-50/50 opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
            <div className="relative z-10">
              <div className="flex items-center gap-3 mb-4">
                <div className="p-2 rounded-lg bg-emerald-100 text-emerald-600">
                  <Activity className="w-5 h-5" />
                </div>
                <h3 className="text-sm font-medium text-slate-500 uppercase tracking-wider">24h Performance</h3>
              </div>
              <div className="flex items-end justify-between">
                <div>
                  <p className="text-4xl font-bold font-mono text-emerald-600">
                    {stats.change_24h_amount}
                  </p>
                  <p className="text-sm font-mono text-emerald-600/80 mt-1">{stats.change_24h_percentage}</p>
                </div>
                <div className="bg-emerald-100 p-2 rounded-full text-emerald-600">
                  <ArrowUpRight className="w-8 h-8" />
                </div>
              </div>
            </div>
          </Card>
        </div>

        {/* Portfolio Value Chart */}
        <Card className="p-6 bg-white/60 border-white/40 backdrop-blur-xl shadow-lg shadow-slate-200/50">
          <div className="flex items-center justify-between mb-8">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-purple-100 text-purple-600">
                <TrendingUp className="w-5 h-5" />
              </div>
              <h3 className="text-lg font-semibold text-slate-900">Portfolio Value (Last 7 Days)</h3>
            </div>
          </div>

          <div className="h-64 px-2">
            {/* Y-axis labels */}
            <div className="flex h-full">
              {/* Chart area */}
              <div className="flex-1 flex gap-2 items-end justify-center">
                {dayLabels.map((dayLabel, index) => {
                  // Find matching data for this day
                  const dataPoint = chartData.find(d => d.day === dayLabel);
                  const hasData = dataPoint !== undefined;
                  const value = hasData ? dataPoint.value : 0;

                  // Calculate height
                  let height = 0;
                  if (hasData && chartMaxValue > chartMinValue) {
                    height = Math.max(10, ((value - chartMinValue) / (chartMaxValue - chartMinValue)) * 100);
                  } else if (hasData) {
                    height = 70; // Default height when there's only one value
                  }

                  return (
                    <div key={index} className="w-[13%] flex flex-col items-center gap-3 group h-full">
                      <div className="w-full relative h-full flex items-end justify-center">
                        {hasData ? (
                          <div
                            className="w-full bg-gradient-to-t from-blue-500 to-cyan-300 rounded-t-md transition-all duration-300 group-hover:opacity-100 group-hover:shadow-[0_0_20px_rgba(34,211,238,0.3)] opacity-80 relative"
                            style={{ height: `${height}%` }}
                          >
                            <div className="absolute -top-8 left-1/2 -translate-x-1/2 bg-slate-800 text-white text-xs py-1 px-2 rounded shadow-lg opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap z-20">
                              ${value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                            </div>
                          </div>
                        ) : (
                          <div className="w-full h-1 bg-slate-200 rounded-full" />
                        )}
                      </div>
                      <span className={`text-xs font-mono transition-colors ${hasData ? 'text-slate-500 group-hover:text-slate-700' : 'text-slate-300'}`}>
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
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Asset Allocation */}
          <Card className="p-6 bg-white/60 border-white/40 backdrop-blur-xl shadow-lg shadow-slate-200/50">
            <h3 className="text-lg font-semibold text-slate-900 mb-6 flex items-center gap-2">
              <span className="w-1 h-6 bg-orange-500 rounded-full shadow-[0_0_8px_rgba(249,115,22,0.4)]" />
              Asset Allocation
            </h3>
            {allocation.length > 0 ? (
              <div className="space-y-4">
                {allocation.map((asset, index) => (
                  <div key={index} className="space-y-2 group">
                    <div className="flex justify-between items-center">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-semibold text-slate-700">{asset.symbol}</span>
                        <span className="text-xs text-slate-400">{asset.name}</span>
                      </div>
                      <div className="text-right">
                        <span className="text-sm font-mono text-slate-700">{asset.percentage.toFixed(1)}%</span>
                        <span className="text-xs text-slate-400 ml-2">
                          ${asset.value_usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </span>
                      </div>
                    </div>
                    <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                      <div
                        className={`h-full ${asset.color} rounded-full transition-all duration-500 group-hover:shadow-[0_0_10px_currentColor]`}
                        style={{ width: `${Math.max(asset.percentage, 1)}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-8 text-slate-400">
                <p className="text-sm">No assets found. Add wallets on the Portfolio page to see allocation.</p>
              </div>
            )}
          </Card>

          {/* Top Movers */}
          <Card className="p-6 bg-white/60 border-white/40 backdrop-blur-xl shadow-lg shadow-slate-200/50">
            <h3 className="text-lg font-semibold text-slate-900 mb-6 flex items-center gap-2">
              <span className="w-1 h-6 bg-cyan-500 rounded-full shadow-[0_0_8px_rgba(6,182,212,0.4)]" />
              Market Prices
            </h3>
            {topMovers.length > 0 ? (
              <div className="space-y-3">
                {topMovers.map((mover, index) => {
                  const getAssetStyle = (symbol: string) => {
                    const styles: Record<string, string> = {
                      'BTC': 'bg-orange-100 text-orange-600',
                      'ETH': 'bg-blue-100 text-blue-600',
                      'SOL': 'bg-purple-100 text-purple-600',
                      'BNB': 'bg-yellow-100 text-yellow-600',
                      'ADA': 'bg-blue-100 text-blue-600',
                      'XRP': 'bg-slate-100 text-slate-600',
                    };
                    return styles[symbol] || 'bg-cyan-100 text-cyan-600';
                  };

                  return (
                    <div
                      key={index}
                      className="flex items-center justify-between p-4 bg-white/50 border border-slate-100 rounded-xl hover:bg-white/80 hover:shadow-md transition-all cursor-pointer group"
                    >
                      <div className="flex items-center gap-3">
                        <div className={`w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold ${getAssetStyle(mover.symbol)}`}>
                          {mover.symbol[0]}
                        </div>
                        <div>
                          <span className="font-medium text-slate-700">{mover.symbol}</span>
                          <span className="text-xs text-slate-400 ml-2">
                            ${mover.price_usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                          </span>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className={`font-mono font-medium ${mover.is_positive ? 'text-emerald-600' : 'text-red-500'}`}>
                          {mover.is_positive ? '+' : ''}{mover.change_24h.toFixed(2)}%
                        </span>
                        {mover.is_positive ? (
                          <ArrowUpRight className="w-4 h-4 text-emerald-600" />
                        ) : (
                          <ArrowDownLeft className="w-4 h-4 text-red-500" />
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="text-center py-8 text-slate-400">
                <p className="text-sm">Loading market data...</p>
              </div>
            )}
          </Card>
        </div>

        {/* Recent Transactions */}
        <Card className="p-6 bg-white/60 border-white/40 backdrop-blur-xl shadow-lg shadow-slate-200/50">
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-lg font-semibold text-slate-900 flex items-center gap-2">
              <Clock className="w-5 h-5 text-slate-400" />
              Recent Transactions
            </h3>
            <a href="#" className="text-blue-600 hover:text-blue-700 text-sm font-medium transition-colors">View All</a>
          </div>

          <div className="space-y-3">
            {recentTransactions.map((tx, index) => (
              <div
                key={index}
                className="flex items-center justify-between p-4 bg-white/50 border border-slate-100 rounded-xl hover:bg-white/80 hover:shadow-md transition-all cursor-pointer group"
              >
                <div className="flex items-center gap-4">
                  <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${tx.type === 'sent'
                    ? 'bg-red-50 text-red-500 border border-red-100'
                    : 'bg-emerald-50 text-emerald-500 border border-emerald-100'
                    }`}>
                    {tx.type === 'sent' ? (
                      <ArrowUpRight className="w-5 h-5" />
                    ) : (
                      <ArrowDownLeft className="w-5 h-5" />
                    )}
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-slate-700">
                        {tx.type === 'sent' ? 'Sent' : 'Received'} {tx.asset}
                      </p>
                      <span className="text-xs px-1.5 py-0.5 rounded bg-slate-100 text-slate-500 font-mono border border-slate-200">
                        {tx.hash}
                      </span>
                    </div>
                    <p className="text-sm text-slate-500">{tx.time}</p>
                  </div>
                </div>
                <p className={`font-mono font-medium ${tx.type === 'sent' ? 'text-red-500' : 'text-emerald-600'
                  }`}>
                  {tx.amount}
                </p>
              </div>
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
};

export default Dashboard;
