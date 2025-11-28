import React from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';
import {
  LayoutDashboard,
  Briefcase,
  ArrowRightLeft,
  LineChart,
  RefreshCw,
  Rocket,
  FolderOpen,
  Coins,
  Settings,
  User
} from 'lucide-react';

interface NavItem {
  icon: React.ElementType;
  label: string;
  href?: string;
}

const Sidebar: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const mainNavItems: NavItem[] = [
    { icon: LayoutDashboard, label: 'Dashboard', href: '/' },
    { icon: Briefcase, label: 'Portfolio', href: '/portfolio' },
    { icon: ArrowRightLeft, label: 'Transactions', href: '/transactions' },
    { icon: LineChart, label: 'Markets', href: '/markets' },
    { icon: RefreshCw, label: 'Swap', href: '/swap' },
  ];

  const secondaryNavItems: NavItem[] = [
    { icon: Rocket, label: 'VC Platform', href: '/vc-platform' },
    { icon: FolderOpen, label: 'Projects', href: '/projects' },
    { icon: Coins, label: 'Investments', href: '/investments' },
  ];

  const footerNavItems: NavItem[] = [
    { icon: Settings, label: 'Settings', href: '/settings' },
    { icon: User, label: 'Profile', href: '/profile' },
  ];

  const isActive = (href: string) => {
    if (href === '/' && location.pathname !== '/') return false;
    return location.pathname.startsWith(href || '');
  };

  const NavItemComponent: React.FC<{ item: NavItem }> = ({ item }) => {
    const Icon = item.icon;
    return (
      <Button
        variant="ghost"
        className={cn(
          "w-full justify-start gap-3 h-10 px-3 text-left font-normal transition-all duration-200",
          isActive(item.href || '')
            ? "bg-primary/10 text-primary hover:bg-primary/15 hover:text-primary"
            : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
        )}
        onClick={() => navigate(item.href || '')}
      >
        <Icon className="w-5 h-5" />
        <span className="text-sm font-medium">{item.label}</span>
      </Button>
    );
  };

  return (
    <div className="w-64 bg-card/50 backdrop-blur-xl border-r border-border h-full flex flex-col pt-4">
      {/* Main Navigation */}
      <div className="px-3 py-2 space-y-1">
        <div className="px-3 mb-2 text-xs font-semibold text-muted-foreground/50 uppercase tracking-wider">
          Menu
        </div>
        {mainNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>

      {/* Secondary Navigation */}
      <div className="px-3 py-2 space-y-1 mt-2">
        <div className="px-3 mb-2 text-xs font-semibold text-muted-foreground/50 uppercase tracking-wider">
          Ventures
        </div>
        {secondaryNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>

      {/* Spacer to push footer to bottom */}
      <div className="flex-1" />

      {/* Divider */}
      <div className="px-6 my-2">
        <Separator className="bg-border/50" />
      </div>

      {/* Footer Navigation */}
      <div className="px-3 pb-6 space-y-1">
        {footerNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>
    </div>
  );
};

export { Sidebar };