import React from 'react';
import { useDashboardData } from './hooks/useDashboardData';
import { StatsCards } from './components/StatsCards';
import { PortfolioChart } from './components/PortfolioChart';
import { AllocationCard } from './components/AllocationCard';
import { RecentTransactions } from './components/RecentTransactions';

const Dashboard: React.FC = () => {
  const {
    stats,
    chartData,
    allocation,
    recentTransactions,
  } = useDashboardData();

  return (
    <div className="min-h-screen p-6 space-y-6 font-sans bg-background selection:bg-primary/20">
      <div className="space-y-6 max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between py-2">
          <div className="text-left">
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
        <StatsCards stats={stats} />

        {/* Portfolio Value Chart */}
        <PortfolioChart chartData={chartData} />

        {/* Asset Allocation */}
        <div className="grid grid-cols-1 gap-4">
          <AllocationCard allocation={allocation} />
        </div>

        {/* Recent Transactions */}
        <RecentTransactions transactions={recentTransactions} />
      </div>
    </div>
  );
};

export default Dashboard;
