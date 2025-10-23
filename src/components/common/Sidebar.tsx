import React from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface NavItem {
  icon: string;
  label: string;
  href?: string;
}

const Sidebar: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const mainNavItems: NavItem[] = [
    { icon: '🏠', label: 'Dashboard', href: '/' },
    { icon: '💼', label: 'Portfolio', href: '/portfolio' },
    { icon: '💸', label: 'Transactions', href: '/transactions' },
    { icon: '📊', label: 'Markets', href: '/markets' },
    { icon: '🔄', label: 'Swap', href: '/swap' },
  ];

  const secondaryNavItems: NavItem[] = [
    { icon: '🚀', label: 'VC Platform', href: '/vc-platform' },
    { icon: '📁', label: 'Projects', href: '/projects' },
    { icon: '💰', label: 'Investments', href: '/investments' },
  ];

  const footerNavItems: NavItem[] = [
    { icon: '⚙️', label: 'Settings', href: '/settings' },
    { icon: '👤', label: 'Profile', href: '/profile' },
  ];

  const isActive = (href: string) => {
    return location.pathname === href;
  };

  const NavItemComponent: React.FC<{ item: NavItem }> = ({ item }) => (
    <Button
      variant="ghost"
      className={cn(
        "w-full justify-start gap-3 h-10 px-3 text-left font-normal",
        isActive(item.href || '') && "bg-accent text-accent-foreground"
      )}
      onClick={() => navigate(item.href || '')}
    >
      <span className="text-lg">{item.icon}</span>
      <span>{item.label}</span>
    </Button>
  );

  return (
    <div className="w-64 bg-background border-r border-border h-full flex flex-col">
      {/* Main Navigation */}
      <div className="p-4 space-y-1">
        {mainNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>

      {/* Divider */}
      <Separator />

      {/* Secondary Navigation */}
      <div className="p-4 space-y-1">
        {secondaryNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>

      {/* Spacer to push footer to bottom */}
      <div className="flex-1" />

      {/* Divider */}
      <Separator />

      {/* Footer Navigation */}
      <div className="p-4 space-y-1">
        {footerNavItems.map((item, index) => (
          <NavItemComponent key={index} item={item} />
        ))}
      </div>
    </div>
  );
};

export { Sidebar };