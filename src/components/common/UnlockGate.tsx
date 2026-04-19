import React, { cloneElement, isValidElement, useEffect, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import {
  SECURITY_STATE_EVENT,
  notifySecurityChanged,
  parseSecurityError,
  securityIsUnlocked,
  securityUnlock,
} from '@/lib/security';
import { toast } from 'sonner';

type GateStatus = 'loading' | 'locked' | 'unlocked';

interface UnlockGateProps {
  children: React.ReactElement<{ onClick?: React.MouseEventHandler<HTMLElement> }>;
  className?: string;
  onUnlockSuccess?: () => void | Promise<void>;
  prompt?: string;
}

const UnlockGate: React.FC<UnlockGateProps> = ({
  children,
  className,
  onUnlockSuccess,
  prompt = 'Unlock required',
}) => {
  const [status, setStatus] = useState<GateStatus>('loading');
  const [token, setToken] = useState('');
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [isUnlocking, setIsUnlocking] = useState(false);

  const handleSecurityError = (error: unknown) => {
    const securityError = parseSecurityError(error);

    switch (securityError) {
      case 'policy_denied':
        setInlineError('policy_denied');
        break;
      case 'operation_not_allowed':
        toast.error('operation_not_allowed');
        break;
      case 'unknown_wallet':
        toast.error('unknown_wallet');
        break;
      case 'locked':
        setStatus('locked');
        break;
      default:
        toast.error(String(error));
    }

    return securityError;
  };

  const refreshStatus = async () => {
    try {
      const unlocked = await securityIsUnlocked();
      setStatus(unlocked ? 'unlocked' : 'locked');
      if (unlocked) {
        setInlineError(null);
      }
    } catch (error) {
      handleSecurityError(error);
      setStatus('locked');
    }
  };

  useEffect(() => {
    void refreshStatus();

    const handleStateChange = () => {
      void refreshStatus();
    };

    window.addEventListener(SECURITY_STATE_EVENT, handleStateChange);
    return () => window.removeEventListener(SECURITY_STATE_EVENT, handleStateChange);
  }, []);

  const handleUnlock = async () => {
    setIsUnlocking(true);
    setInlineError(null);

    try {
      await securityUnlock(token);
      setStatus('unlocked');
      setToken('');
      notifySecurityChanged();

      if (onUnlockSuccess) {
        await onUnlockSuccess();
      }
    } catch (error) {
      handleSecurityError(error);
    } finally {
      setIsUnlocking(false);
    }
  };

  if (status === 'loading') {
    return <div className={cn('h-9 w-40 animate-pulse rounded-md bg-muted/50', className)} />;
  }

  if (status === 'locked') {
    return (
      <div className={cn('flex min-w-[220px] flex-col gap-1 rounded-md border border-border/50 bg-muted/20 p-2', className)}>
        <span className="text-[10px] font-medium text-muted-foreground">{prompt}</span>
        <div className="flex items-center gap-2">
          <Input
            type="password"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            placeholder="Passphrase"
            className="h-8 text-xs"
          />
          <Button
            type="button"
            size="sm"
            onClick={() => void handleUnlock()}
            disabled={isUnlocking}
          >
            {isUnlocking ? '...' : 'Unlock'}
          </Button>
        </div>
        {inlineError && (
          <p className="text-[10px] text-destructive">{inlineError}</p>
        )}
      </div>
    );
  }

  if (!isValidElement(children)) {
    return null;
  }

  const originalOnClick = children.props.onClick;

  return cloneElement(children, {
    onClick: async (event: React.MouseEvent<HTMLElement>) => {
      try {
        const unlocked = await securityIsUnlocked();
        if (!unlocked) {
          event.preventDefault();
          event.stopPropagation();
          setStatus('locked');
          return;
        }

        await Promise.resolve(originalOnClick?.(event));
      } catch (error) {
        handleSecurityError(error);
      }
    },
  });
};

export { UnlockGate };
