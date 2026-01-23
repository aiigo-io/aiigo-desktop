import { useState, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    DashboardStats,
    PortfolioHistoryPoint,
    AssetAllocation,
    UnifiedTransaction,
    ChartDataPoint
} from '../types';

export const useDashboardData = () => {
    const [stats, setStats] = useState<DashboardStats>({
        total_balance_usd: '$0.00',
        total_balance_btc: '≈ 0.00 BTC',
        change_24h_amount: '+$0.00',
        change_24h_percentage: '+0.00%'
    });
    const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
    const [allocation, setAllocation] = useState<AssetAllocation[]>([]);
    const [recentTransactions, setRecentTransactions] = useState<UnifiedTransaction[]>([]);
    const [loading, setLoading] = useState(true);

    const loadRecentTransactions = useCallback(async () => {
        try {
            const txs = await invoke<UnifiedTransaction[]>('get_unified_recent_transactions');
            setRecentTransactions(txs);
        } catch (error) {
            console.error('Failed to load recent transactions:', error);
        }
    }, []);

    const formatChartData = (history: PortfolioHistoryPoint[]) => {
        return history.map(point => {
            const date = new Date(point.date);
            const dayName = date.toLocaleDateString('en-US', { weekday: 'short' });
            return {
                day: dayName,
                value: point.value
            };
        });
    };

    const loadData = useCallback(async () => {
        setLoading(true);
        try {
            // 1. Load cached data immediately
            const cachedStats = await invoke<DashboardStats>('get_dashboard_stats');
            setStats(cachedStats);

            const history = await invoke<PortfolioHistoryPoint[]>('get_portfolio_history');
            if (history && history.length > 0) {
                setChartData(formatChartData(history));
            }

            const allocationData = await invoke<AssetAllocation[]>('get_asset_allocation');
            setAllocation(allocationData);

            await loadRecentTransactions();

            // 2. Refresh data in background
            const freshStats = await invoke<DashboardStats>('refresh_dashboard_stats');
            setStats(freshStats);

            const updatedHistory = await invoke<PortfolioHistoryPoint[]>('get_portfolio_history');
            if (updatedHistory && updatedHistory.length > 0) {
                setChartData(formatChartData(updatedHistory));
            }

            const updatedAllocation = await invoke<AssetAllocation[]>('get_asset_allocation');
            setAllocation(updatedAllocation);
        } catch (error) {
            console.error('Failed to load dashboard data:', error);
        } finally {
            setLoading(false);
        }
    }, [loadRecentTransactions]);

    useEffect(() => {
        loadData();
    }, [loadData]);

    const syncedStats = useMemo(() => {
        const totalUsd = allocation.reduce((acc, curr) => acc + curr.value_usd, 0);

        const formatCurrency = (value: number) => {
            return '$' + value.toLocaleString('en-US', {
                minimumFractionDigits: 2,
                maximumFractionDigits: 2
            });
        };

        let totalBtc = stats.total_balance_btc;
        try {
            // Try to keep BTC in sync by using the price ratio from existing stats
            const currentUsdStr = stats.total_balance_usd.replace(/[$,]/g, '');
            const currentBtcStr = stats.total_balance_btc.replace(/[≈\sBTC]/g, '');
            const currentUsd = parseFloat(currentUsdStr);
            const currentBtc = parseFloat(currentBtcStr);

            if (currentUsd > 0 && currentBtc > 0) {
                const btcPrice = currentUsd / currentBtc;
                const newBtcBalance = totalUsd / btcPrice;
                totalBtc = `≈ ${newBtcBalance.toFixed(4)} BTC`;
            }
        } catch (e) {
            // Fallback to original if parsing fails
        }

        return {
            ...stats,
            total_balance_usd: formatCurrency(totalUsd),
            total_balance_btc: totalBtc
        };
    }, [stats, allocation]);

    return {
        stats: syncedStats,
        chartData,
        allocation,
        recentTransactions,
        loading,
        refresh: loadData
    };
};
