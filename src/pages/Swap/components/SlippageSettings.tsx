import React, { useState } from 'react';
import { X, AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

interface SlippageSettingsProps {
    slippage: number;
    onSlippageChange: (slippage: number) => void;
    onClose: () => void;
}

const PRESET_SLIPPAGES = [0.5, 1, 2];
const MIN_SLIPPAGE = 0.05;
const MAX_SLIPPAGE = 50;
const HIGH_SLIPPAGE_WARNING = 5;

export const SlippageSettings: React.FC<SlippageSettingsProps> = ({
    slippage,
    onSlippageChange,
    onClose,
}) => {
    const [customValue, setCustomValue] = useState<string>(
        PRESET_SLIPPAGES.includes(slippage) ? '' : slippage.toString()
    );
    const [error, setError] = useState<string>('');

    const handlePresetClick = (value: number) => {
        setCustomValue('');
        setError('');
        onSlippageChange(value);
    };

    const handleCustomChange = (value: string) => {
        setCustomValue(value);

        if (!value) {
            setError('');
            return;
        }

        const numValue = parseFloat(value);

        if (isNaN(numValue)) {
            setError('Please enter a valid number');
            return;
        }

        if (numValue < MIN_SLIPPAGE) {
            setError(`Minimum slippage is ${MIN_SLIPPAGE}%`);
            return;
        }

        if (numValue > MAX_SLIPPAGE) {
            setError(`Maximum slippage is ${MAX_SLIPPAGE}%`);
            return;
        }

        setError('');
        onSlippageChange(numValue);
    };

    const isHighSlippage = slippage > HIGH_SLIPPAGE_WARNING;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
            <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md mx-4 p-6 space-y-4">
                {/* Header */}
                <div className="flex items-center justify-between">
                    <h3 className="text-lg font-semibold text-slate-800">Slippage Settings</h3>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={onClose}
                        className="text-slate-500 hover:text-slate-700"
                    >
                        <X className="size-4" />
                    </Button>
                </div>

                {/* Description */}
                <p className="text-sm text-slate-600">
                    Your transaction will revert if the price changes unfavorably by more than this percentage.
                </p>

                {/* Preset Buttons */}
                <div className="space-y-2">
                    <label className="text-xs font-medium text-slate-700">Preset Values</label>
                    <div className="grid grid-cols-3 gap-2">
                        {PRESET_SLIPPAGES.map((preset) => (
                            <Button
                                key={preset}
                                variant={slippage === preset && !customValue ? 'default' : 'outline'}
                                onClick={() => handlePresetClick(preset)}
                                className={`h-10 ${slippage === preset && !customValue
                                        ? 'bg-blue-600 text-white hover:bg-blue-700'
                                        : 'bg-white hover:bg-slate-50'
                                    }`}
                            >
                                {preset}%
                            </Button>
                        ))}
                    </div>
                </div>

                {/* Custom Input */}
                <div className="space-y-2">
                    <label className="text-xs font-medium text-slate-700">Custom Slippage</label>
                    <div className="relative">
                        <Input
                            type="number"
                            placeholder="Enter custom slippage"
                            value={customValue}
                            onChange={(e) => handleCustomChange(e.target.value)}
                            min={MIN_SLIPPAGE}
                            max={MAX_SLIPPAGE}
                            step={0.1}
                            className={`pr-8 ${error ? 'border-red-500 focus-visible:ring-red-500' : ''}`}
                        />
                        <span className="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-slate-500">
                            %
                        </span>
                    </div>
                    {error && (
                        <p className="text-xs text-red-500">{error}</p>
                    )}
                </div>

                {/* High Slippage Warning */}
                {isHighSlippage && (
                    <div className="flex items-start gap-2 p-3 rounded-xl bg-orange-50 border border-orange-200">
                        <AlertTriangle className="size-4 mt-0.5 text-orange-500 shrink-0" />
                        <div className="flex-1 text-xs">
                            <p className="font-semibold text-orange-700">High Slippage Warning</p>
                            <p className="text-orange-600">
                                Your transaction may be frontrun due to high slippage tolerance.
                            </p>
                        </div>
                    </div>
                )}

                {/* Current Value Display */}
                <div className="pt-2 border-t border-slate-200">
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-slate-600">Current Slippage:</span>
                        <span className="font-semibold text-slate-800">{slippage}%</span>
                    </div>
                </div>

                {/* Close Button */}
                <Button
                    onClick={onClose}
                    className="w-full h-10 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-xl"
                >
                    Done
                </Button>
            </div>
        </div>
    );
};
