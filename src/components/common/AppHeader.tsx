import React, { useState, useEffect } from 'react';
import dayjs from 'dayjs';

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
    <header className='h-12 bg-foreground px-6 flex items-center justify-between text-primary-foreground select-none'>
      <div className="flex items-center gap-4">
        <img className='w-9 h-9' src="/favicon.svg" alt="AIIGO" />
        <h1 className="text-xl font-bold">AIIGO Platform</h1>
      </div>
      
      <div className="flex items-center gap-4 text-sm">
        <div className="text-right">
          <div className="font-mono text-lg font-semibold">
            {formatTime(currentTime)}
          </div>
          <div className="text-xs opacity-80">
            {formatDate(currentTime)}
          </div>
        </div>
      </div>
    </header>
  )
}

export { AppHeader }