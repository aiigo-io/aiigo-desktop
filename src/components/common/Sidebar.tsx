import React from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';
import {
  LayoutDashboard,
  ArrowRightLeft,
  LineChart,
  RefreshCw,
  Rocket,
  FolderOpen,
  Coins,
  Settings,
  User,
  Wallet,
  Cpu,
  ChevronDown
} from 'lucide-react';

interface NavItem {
  icon: React.ElementType;
  label: string;
  href?: string;
  children?: {
    label: string;
    href: string;
    icon?: React.ElementType;
  }[];
}

const Sidebar: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const mainNavItems: NavItem[] = [
    { icon: LayoutDashboard, label: 'Dashboard', href: '/' },
    { icon: Wallet, label: 'Portfolio', href: '/portfolio' },
    { icon: RefreshCw, label: 'Swap', href: '/swap' },
    { icon: ArrowRightLeft, label: 'Transactions', href: '/transactions' },
  ];

  const secondaryNavItems: NavItem[] = [
    { icon: Rocket, label: 'VC Platform', href: '/vc-platform' },
    {
      icon: FolderOpen,
      label: 'Projects',
      children: [
        { label: 'Computing Power', href: '/projects/computing-power', icon: Cpu },
      ]
    },
    { icon: Coins, label: 'Investments', href: '/investments' },
  ];

  const footerNavItems: NavItem[] = [
    { icon: Settings, label: 'Settings', href: '/settings' },
    { icon: User, label: 'Profile', href: '/profile' },
  ];

  const isActive = (href: string) => {
    if (href === '/' && location.pathname !== '/') return false;
    return location.pathname === href || location.pathname.startsWith(href + '/');
  };

  const NavItemComponent: React.FC<{ item: NavItem; isChild?: boolean }> = ({ item, isChild = false }) => {
    const Icon = item.icon;
    const active = item.href ? isActive(item.href) : false;
    const hasChildren = item.children && item.children.length > 0;

    const handleClick = () => {
      if (item.href) {
        navigate(item.href);
      }
    };

    return (
      <div className="space-y-0.5">
        <Button
          variant="ghost"
          className={cn(
            "w-full justify-start gap-3 h-9 px-3 text-left font-normal transition-all duration-200 group relative overflow-hidden rounded-md",
            active
              ? "bg-sidebar-accent text-sidebar-primary"
              : "text-sidebar-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-primary",
            isChild && "h-8 text-xs"
          )}
          onClick={handleClick}
        >
          {active && !isChild && (
            <div className="absolute left-0 top-1.5 bottom-1.5 w-0.5 bg-sidebar-primary rounded-full" />
          )}
          {Icon && (
            <Icon className={cn(
              "w-4 h-4 transition-colors shrink-0",
              active ? "text-sidebar-primary" : "text-sidebar-foreground/70 group-hover:text-sidebar-primary",
              isChild && "w-3.5 h-3.5"
            )} />
          )}
          <span className={cn("flex-1 text-sm font-medium tracking-tight truncate", isChild && "text-xs")}>
            {item.label}
          </span>
          {hasChildren && (
            <ChevronDown className="w-3.5 h-3.5 text-sidebar-foreground/50 transition-colors group-hover:text-sidebar-primary" />
          )}
        </Button>

        {hasChildren && (
          <div className="space-y-0.5 mt-0.5 ml-5 pl-4 border-l border-sidebar-border/50">
            {item.children?.map((child, idx) => (
              <NavItemComponent
                key={idx}
                item={{
                  label: child.label,
                  href: child.href,
                  icon: child.icon || FolderOpen
                }}
                isChild
              />
            ))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="w-60 bg-sidebar border-r border-sidebar-border h-full flex flex-col pt-4 overflow-y-auto no-scrollbar">
      {/* Main Navigation */}
      <div className="px-2 py-2 space-y-0.5">
        <div className="px-3 mb-2 text-[10px] font-bold text-sidebar-foreground/60 uppercase tracking-widest font-mono">
          Platform
        </div>
        {mainNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>

      {/* Secondary Navigation */}
      <div className="px-2 py-2 space-y-0.5 mt-4">
        <div className="px-3 mb-2 text-[10px] font-bold text-sidebar-foreground/60 uppercase tracking-widest font-mono">
          Ventures
        </div>
        {secondaryNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Divider */}
      <div className="px-4 my-2">
        <Separator className="bg-sidebar-border" />
      </div>

      {/* Footer Navigation */}
      <div className="px-2 pb-6 space-y-0.5">
        <div className="px-3 mb-2 text-[10px] font-bold text-sidebar-foreground/60 uppercase tracking-widest font-mono">
          System
        </div>
        {footerNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>
    </div>
  );
};

export { Sidebar };