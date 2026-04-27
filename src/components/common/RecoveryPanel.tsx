import React from 'react';
import { AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getWalletRecoveryToneClass, type WalletRecoveryGuidance } from '@/lib/wallet-recovery';

interface RecoveryPanelProps {
  guidance: WalletRecoveryGuidance;
  className?: string;
}

const RecoveryPanel: React.FC<RecoveryPanelProps> = ({ guidance, className }) => {
  return (
    <div className={cn('rounded-xl border px-4 py-3 text-sm', getWalletRecoveryToneClass(guidance.tone), className)}>
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 size-4 shrink-0" />
        <div className="min-w-0 space-y-2">
          <div>
            <p className="font-semibold">{guidance.title}</p>
            <p className="mt-1 leading-relaxed opacity-90">{guidance.summary}</p>
          </div>
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-wide opacity-80">Recovery Action</p>
            <ul className="mt-1 list-disc space-y-1 pl-4 text-xs leading-relaxed opacity-90">
              {guidance.actions.map((action) => (
                <li key={action}>{action}</li>
              ))}
            </ul>
          </div>
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-wide opacity-80">Raw Error</p>
            <p className="mt-1 break-all font-mono text-[11px] opacity-80">{guidance.rawError}</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default RecoveryPanel;