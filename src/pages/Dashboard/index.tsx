import React from 'react';
import { Card } from '@/components/ui/card';
import { ArrowUpRight, ArrowDownLeft } from 'lucide-react';

const Dashboard: React.FC = () => {
  // Mock data
  const stats = {
    totalBalance: {
      usd: '$125,487.32',
      btc: '≈ 3.45 BTC'
    },
    change24h: {
      amount: '+$4,231.21',
      percentage: '+3.49%'
    }
  };

  const chartData = [
    { day: 'Mon', value: 120000 },
    { day: 'Tue', value: 115000 },
    { day: 'Wed', value: 128000 },
    { day: 'Thu', value: 122000 },
    { day: 'Fri', value: 135000 },
    { day: 'Sat', value: 132000 },
    { day: 'Sun', value: 140000 },
  ];

  const maxValue = Math.max(...chartData.map(d => d.value));
  const minValue = Math.max(0, Math.min(...chartData.map(d => d.value)) - 1000);

  const allocation = [
    { name: 'BTC', percentage: 45, color: 'bg-orange-500' },
    { name: 'ETH', percentage: 30, color: 'bg-blue-500' },
    { name: 'Other', percentage: 25, color: 'bg-purple-500' }
  ];

  const topMovers = [
    { symbol: 'BTC', change: '+5.2%', isPositive: true },
    { symbol: 'ETH', change: '+3.1%', isPositive: true },
    { symbol: 'SOL', change: '-2.4%', isPositive: false }
  ];

  const recentTransactions = [
    { type: 'sent', asset: 'BTC', amount: '-0.05 BTC', time: '2 hours ago' },
    { type: 'received', asset: 'ETH', amount: '+1.2 ETH', time: '5 hours ago' }
  ];

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-foreground">Dashboard</h1>
        <div className="flex gap-2">
          <select className="px-4 py-2 bg-background border border-border rounded-lg text-foreground text-sm">
            <option>Last 7 days</option>
            <option>Last 30 days</option>
            <option>Last 3 months</option>
            <option>Last year</option>
          </select>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-2 gap-6">
        {/* Total Balance Card */}
        <Card className="p-6">
          <div className="space-y-2">
            <h3 className="text-sm font-medium text-muted-foreground">Total Balance</h3>
            <div className="space-y-1">
              <p className="text-3xl font-bold text-foreground">{stats.totalBalance.usd}</p>
              <p className="text-sm text-muted-foreground">{stats.totalBalance.btc}</p>
            </div>
          </div>
        </Card>

        {/* 24h Change Card */}
        <Card className="p-6">
          <div className="space-y-2">
            <h3 className="text-sm font-medium text-muted-foreground">24h Change</h3>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-3xl font-bold text-green-500">{stats.change24h.amount}</p>
                <p className="text-sm text-green-500 font-medium">{stats.change24h.percentage}</p>
              </div>
              <ArrowUpRight className="w-8 h-8 text-green-500" />
            </div>
          </div>
        </Card>
      </div>

      {/* Portfolio Value Chart */}
      <Card className="p-6">
        <h3 className="text-lg font-semibold text-foreground mb-6">Portfolio Value Chart (7 days)</h3>
        
        <div className="flex justify-between h-48 gap-2">
          {chartData.map((data, index) => {
            const height = ((data.value - minValue) / (maxValue - minValue)) * 100;
            return (
              <div key={index} className="flex-1 flex flex-col items-center gap-2 justify-end">
                <div 
                  className="w-full bg-gradient-to-t from-blue-500 to-blue-300 rounded-t transition-all hover:opacity-80 cursor-pointer"
                  style={{ height: `${height}%` }}
                  title={`$${data.value.toLocaleString()}`}
                />
                <span className="text-xs text-muted-foreground">{data.day}</span>
              </div>
            );
          })}
        </div>

        <div className="mt-4 pt-4 border-t border-border text-xs text-muted-foreground">
          <p>Max: ${maxValue.toLocaleString()} | Min: ${minValue.toLocaleString()}</p>
        </div>
      </Card>

      {/* Asset Allocation and Top Movers */}
      <div className="grid grid-cols-2 gap-6">
        {/* Asset Allocation */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-foreground mb-6">Asset Allocation</h3>
          <div className="space-y-4">
            {allocation.map((asset, index) => (
              <div key={index} className="space-y-2">
                <div className="flex justify-between items-center">
                  <span className="text-sm font-medium text-foreground">{asset.name}</span>
                  <span className="text-sm font-semibold text-foreground">{asset.percentage}%</span>
                </div>
                <div className="w-full bg-muted rounded-full h-2 overflow-hidden">
                  <div 
                    className={`h-full ${asset.color} rounded-full`}
                    style={{ width: `${asset.percentage}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </Card>

        {/* Top Movers */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-foreground mb-6">Top Movers</h3>
          <div className="space-y-3">
            {topMovers.map((mover, index) => (
              <div key={index} className="flex items-center justify-between p-3 bg-muted rounded-lg">
                <span className="font-medium text-foreground">{mover.symbol}</span>
                <div className="flex items-center gap-2">
                  <span className={`font-semibold ${mover.isPositive ? 'text-green-500' : 'text-red-500'}`}>
                    {mover.change}
                  </span>
                  {mover.isPositive ? (
                    <ArrowUpRight className="w-4 h-4 text-green-500" />
                  ) : (
                    <ArrowDownLeft className="w-4 h-4 text-red-500" />
                  )}
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>

      {/* Recent Transactions */}
      <Card className="p-6">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-lg font-semibold text-foreground">Recent Transactions</h3>
          <a href="#" className="text-primary hover:underline text-sm font-medium">View All</a>
        </div>
        
        <div className="space-y-3">
          {recentTransactions.map((tx, index) => (
            <div key={index} className="flex items-center justify-between p-4 bg-muted rounded-lg hover:bg-muted/80 transition-colors cursor-pointer">
              <div className="flex items-center gap-4">
                <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${tx.type === 'sent' ? 'bg-red-100 dark:bg-red-900/30' : 'bg-green-100 dark:bg-green-900/30'}`}>
                  {tx.type === 'sent' ? (
                    <ArrowUpRight className={`w-5 h-5 ${tx.type === 'sent' ? 'text-red-500' : 'text-green-500'}`} />
                  ) : (
                    <ArrowDownLeft className="w-5 h-5 text-green-500" />
                  )}
                </div>
                <div>
                  <p className="font-medium text-foreground">
                    {tx.type === 'sent' ? 'Sent' : 'Received'} {tx.asset}
                  </p>
                  <p className="text-sm text-muted-foreground">{tx.time}</p>
                </div>
              </div>
              <p className={`font-semibold ${tx.type === 'sent' ? 'text-red-500' : 'text-green-500'}`}>
                {tx.amount}
              </p>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
};

export default Dashboard;
