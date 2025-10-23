import React from 'react';
import { Outlet } from 'react-router-dom';

import { Sidebar } from '@/components/common/Sidebar';
import { AppHeader } from '@/components/common/AppHeader';
const AppLayout: React.FC = () => {
  return (
    <div className="h-screen w-screen overflow-hidden flex flex-col">
      <AppHeader />
      <main className="flex-1 overflow-auto flex items-stretch">
        <Sidebar />
        <div className="flex-1 overflow-auto relative">
          <Outlet />
        </div>
      </main>
    </div>
  );
};

export { AppLayout };