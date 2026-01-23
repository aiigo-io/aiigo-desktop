import React, { useMemo } from 'react';
import { Card } from '@/components/ui/card';
import { TrendingUp } from 'lucide-react';
import { ChartDataPoint } from '../types';

interface PortfolioChartProps {
    chartData: ChartDataPoint[];
}

export const PortfolioChart: React.FC<PortfolioChartProps> = ({ chartData }) => {
    const dayLabels = useMemo(() => {
        const labels = [];
        for (let i = 6; i >= 0; i--) {
            const date = new Date();
            date.setDate(date.getDate() - i);
            labels.push(date.toLocaleDateString('en-US', { weekday: 'short' }));
        }
        return labels;
    }, []);

    const chartValueLimits = useMemo(() => {
        const maxValue = chartData.length > 0 ? Math.max(...chartData.map(d => d.value)) : 0;
        const minValue = chartData.length > 0 ? Math.min(...chartData.map(d => d.value)) : 0;

        const valueRange = maxValue - minValue;
        const baseValue = Math.max(maxValue, 1);
        const padding = valueRange > 0 ? valueRange * 0.1 : baseValue * 0.2;
        const chartMinValue = Math.max(0, minValue - padding);
        const chartMaxValue = maxValue + padding;

        return { chartMinValue, chartMaxValue };
    }, [chartData]);

    const { chartMinValue, chartMaxValue } = chartValueLimits;

    return (
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
    );
};
