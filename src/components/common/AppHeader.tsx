import React, { useState, useEffect } from 'react';
import dayjs from 'dayjs';
import { Lock } from 'lucide-react';

import { useSecuritySession } from '@/components/common/SecuritySession';
import { Button } from '@/components/ui/button';
import { notifySecurityChanged, securityLock } from '@/lib/security';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

const AppHeader: React.FC = () => {
  const [currentTime, setCurrentTime] = useState(dayjs());
  const [isLocking, setIsLocking] = useState(false);
  const { status, backendState } = useSecuritySession();

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(dayjs());
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  const formatTime = (date: dayjs.Dayjs) => {
    return date.format('HH:mm:ss');
  };

  const formatDate = (date: dayjs.Dayjs) => {
    return date.format('MMM DD, YYYY');
  };

  const handleLock = async () => {
    setIsLocking(true);
    try {
      await securityLock();
      notifySecurityChanged();
      toast.success('Wallet locked');
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsLocking(false);
    }
  };

  return (
    <header className={cn(
      "h-16 px-6 flex items-center justify-between select-none transition-colors duration-200",
      "bg-background/50 backdrop-blur-xl border-b border-sidebar-border",
      "text-foreground"
    )}>
      <div className="flex items-center gap-4">
        <img className='w-8 h-8' src="/favicon.png" alt="AIIGO" />
        <h1 className="text-lg font-bold tracking-tight bg-gradient-to-r from-white to-white/60 bg-clip-text text-transparent">AIIGO Platform</h1>
      </div>

      <div className="flex items-center gap-4">
        {backendState?.degraded && (
          <div className="rounded-full border border-amber-500/30 bg-amber-500/10 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em] text-amber-200">
            Degraded Security
          </div>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={() => void handleLock()}
          disabled={isLocking || status !== 'unlocked'}
          className="gap-2 rounded-2xl border-border/60 bg-[#141C27] px-4 text-slate-200 hover:bg-[#192331]"
        >
          <Lock className="w-4 h-4" />
          {isLocking ? 'Locking...' : 'Lock'}
        </Button>
        <div className="text-right">
          <div className="font-mono text-sm font-medium tabular-nums leading-none mb-1 text-foreground/90">
            {formatTime(currentTime)}
          </div>
          <div className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest font-mono">
            {formatDate(currentTime)}
          </div>
        </div>
      </div>
    </header>
  )
}

export { AppHeader }
