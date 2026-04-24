import { useState, useEffect, useCallback } from 'react';
import { invoke, isTauriRuntimeAvailable } from '@/lib/tauri';
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
        change_24h_percentage: '+0.00%',
        freshness: {
            status: 'unavailable',
            updated_at: null,
            failed_sources: [],
        },
        valuation_status: 'valued',
        unpriced_asset_count: 0,
    });
    const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
    const [allocation, setAllocation] = useState<AssetAllocation[]>([]);
    const [recentTransactions, setRecentTransactions] = useState<UnifiedTransaction[]>([]);
    const [loading, setLoading] = useState(true);

    const loadRecentTransactions = useCallback(async () => {
        if (!isTauriRuntimeAvailable()) {
            setRecentTransactions([]);
            return;
        }

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
        if (!isTauriRuntimeAvailable()) {
            setStats({
                total_balance_usd: '$0.00',
                total_balance_btc: '≈ 0.00 BTC',
                change_24h_amount: '+$0.00',
                change_24h_percentage: '+0.00%',
                freshness: {
                    status: 'unavailable',
                    updated_at: null,
                    failed_sources: [],
                },
                valuation_status: 'valued',
                unpriced_asset_count: 0,
            });
            setChartData([]);
            setAllocation([]);
            setRecentTransactions([]);
            setLoading(false);
            return;
        }

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

    return {
        stats,
        chartData,
        allocation,
        recentTransactions,
        loading,
        refresh: loadData
    };
};
