import React, { useState, useEffect } from 'react';
import dayjs from 'dayjs';
import { cn } from '@/lib/utils';

const AppHeader: React.FC = () => {
  const [currentTime, setCurrentTime] = useState(dayjs());

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

  return (
    <header className={cn(
      "h-16 px-6 flex items-center justify-between select-none transition-colors duration-200",
      "bg-background/80 backdrop-blur-md border-b border-border/50",
      "text-foreground"
    )}>
      <div className="flex items-center gap-4">
        <img className='w-9 h-9' src="/favicon.png" alt="AIIGO" />
        <h1 className="text-xl font-bold tracking-tight">AIIGO Platform</h1>
      </div>

      <div className="flex items-center gap-4">
        <div className="text-right">
          <div className="font-mono text-sm font-medium tabular-nums leading-none mb-1">
            {formatTime(currentTime)}
          </div>
          <div className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
            {formatDate(currentTime)}
          </div>
        </div>
      </div>
    </header>
  )
}

export { AppHeader }