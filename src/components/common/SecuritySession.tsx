import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  SECURITY_STATE_EVENT,
  notifySecurityChanged,
  parseSecurityError,
  securityIsUnlocked,
  securityUnlock,
} from '@/lib/security';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

type GateStatus = 'loading' | 'locked' | 'unlocked';
type UnlockReason = 'locked' | 'expired';

interface PendingAction {
  prompt?: string;
  onUnlockSuccess?: () => void | Promise<void>;
  reason: UnlockReason;
}

interface SecuritySessionContextValue {
  status: GateStatus;
  requestUnlock: (action?: Omit<PendingAction, 'reason'> & { reason?: UnlockReason }) => Promise<void>;
  refreshStatus: () => Promise<boolean>;
}

const SecuritySessionContext = createContext<SecuritySessionContextValue | null>(null);

const statusClassName = {
  loading: 'border-border/60 text-muted-foreground',
  locked: 'border-amber-500/30 bg-amber-500/5 text-amber-200',
  unlocked: 'border-emerald-500/30 bg-emerald-500/5 text-emerald-200',
} satisfies Record<GateStatus, string>;

const statusDotClassName = {
  loading: 'bg-muted-foreground/50',
  locked: 'bg-amber-400',
  unlocked: 'bg-emerald-400',
} satisfies Record<GateStatus, string>;

const statusLabel = {
  loading: 'Checking',
  locked: 'Locked',
  unlocked: 'Unlocked',
} satisfies Record<GateStatus, string>;

const SecuritySessionProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [status, setStatus] = useState<GateStatus>('loading');
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [token, setToken] = useState('');
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [isUnlocking, setIsUnlocking] = useState(false);
  const actionRef = useRef<PendingAction | null>(null);

  const handleSecurityError = useCallback((error: unknown) => {
    const securityError = parseSecurityError(error);

    switch (securityError) {
      case 'policy_denied':
        setInlineError('Enter a non-empty unlock token.');
        break;
      case 'operation_not_allowed':
        toast.error('Unlock is temporarily unavailable. Try again.');
        break;
      case 'unknown_wallet':
        toast.error('Wallet was not found. Refresh and try again.');
        break;
      case 'locked':
        setStatus('locked');
        break;
      default:
        toast.error(String(error));
    }

    return securityError;
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      const unlocked = await securityIsUnlocked();
      setStatus(unlocked ? 'unlocked' : 'locked');
      if (unlocked) {
        setInlineError(null);
      }
      return unlocked;
    } catch (error) {
      handleSecurityError(error);
      setStatus('locked');
      return false;
    }
  }, [handleSecurityError]);

  useEffect(() => {
    void refreshStatus();

    const handleStateChange = () => {
      void refreshStatus();
    };

    const timer = window.setInterval(() => {
      void refreshStatus();
    }, 5000);

    window.addEventListener(SECURITY_STATE_EVENT, handleStateChange);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener(SECURITY_STATE_EVENT, handleStateChange);
    };
  }, [refreshStatus]);

  const requestUnlock = useCallback<SecuritySessionContextValue['requestUnlock']>(async (action = {}) => {
    const unlocked = status === 'unlocked' ? true : await refreshStatus();
    if (unlocked) {
      await action.onUnlockSuccess?.();
      return;
    }

    const nextAction: PendingAction = {
      prompt: action.prompt,
      onUnlockSuccess: action.onUnlockSuccess,
      reason: action.reason ?? 'locked',
    };

    actionRef.current = nextAction;
    setPendingAction(nextAction);
    setToken('');
    setInlineError(null);
    setIsDialogOpen(true);
  }, [refreshStatus, status]);

  const handleUnlock = useCallback(async () => {
    setIsUnlocking(true);
    setInlineError(null);

    try {
      await securityUnlock(token);
      setStatus('unlocked');
      setIsDialogOpen(false);
      setToken('');
      notifySecurityChanged();

      const nextAction = actionRef.current;
      actionRef.current = null;
      setPendingAction(null);

      if (nextAction?.onUnlockSuccess) {
        await nextAction.onUnlockSuccess();
      }
    } catch (error) {
      handleSecurityError(error);
    } finally {
      setIsUnlocking(false);
    }
  }, [handleSecurityError, token]);

  const handleOpenChange = useCallback((open: boolean) => {
    setIsDialogOpen(open);
    if (!open) {
      actionRef.current = null;
      setPendingAction(null);
      setToken('');
      setInlineError(null);
    }
  }, []);

  const contextValue = useMemo<SecuritySessionContextValue>(() => ({
    status,
    requestUnlock,
    refreshStatus,
  }), [refreshStatus, requestUnlock, status]);

  const dialogTitle = pendingAction?.reason === 'expired' ? 'Session expired' : 'Unlock wallet';
  const dialogDescription = pendingAction?.reason === 'expired'
    ? 'Unlock again to continue the same action.'
    : 'Required for export and signing actions.';

  return (
    <SecuritySessionContext.Provider value={contextValue}>
      {children}
      <Dialog open={isDialogOpen} onOpenChange={handleOpenChange}>
        <DialogContent className="border-border/60 bg-[#171E29] text-foreground sm:max-w-[492px]">
          <DialogHeader className="space-y-3">
            <div className="flex items-center gap-3">
              <div
                className={cn(
                  'inline-flex items-center gap-2 rounded-full border px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em]',
                  pendingAction?.reason === 'expired'
                    ? 'border-amber-500/30 bg-amber-500/5 text-amber-200'
                    : 'border-amber-500/30 bg-amber-500/5 text-amber-200'
                )}
              >
                <span className="h-2 w-2 rounded-full bg-amber-400" />
                Locked
              </div>
            </div>
            <DialogTitle className="text-3xl font-semibold tracking-tight text-white">
              {dialogTitle}
            </DialogTitle>
            <DialogDescription className="text-base text-slate-400">
              {dialogDescription}
            </DialogDescription>
            {pendingAction?.prompt && (
              <p className="text-sm text-slate-300">{pendingAction.prompt}</p>
            )}
          </DialogHeader>

          <div className="space-y-6">
            <div className="space-y-2">
              <label className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">
                Unlock Token
              </label>
              <Input
                type="password"
                value={token}
                onChange={(event) => setToken(event.target.value)}
                placeholder="Enter unlock token"
                className="h-14 border-border/70 bg-[#121924] text-lg text-white placeholder:text-slate-500"
              />
              {inlineError && (
                <p className="text-sm text-destructive">{inlineError}</p>
              )}
            </div>

            <div className="flex items-center justify-between gap-3">
              <Button
                type="button"
                variant="outline"
                className="h-12 flex-1 border-border/70 bg-[#131C27] text-base text-slate-200 hover:bg-[#1A2431]"
                onClick={() => handleOpenChange(false)}
              >
                Cancel
              </Button>
              <Button
                type="button"
                className="h-12 flex-1 bg-cyan-400 text-base font-semibold text-slate-950 hover:bg-cyan-300"
                onClick={() => void handleUnlock()}
                disabled={isUnlocking}
              >
                {isUnlocking ? 'Unlocking...' : 'Unlock'}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </SecuritySessionContext.Provider>
  );
};

const useSecuritySession = () => {
  const context = useContext(SecuritySessionContext);
  if (!context) {
    throw new Error('useSecuritySession must be used within SecuritySessionProvider');
  }
  return context;
};

const SessionBadge: React.FC = () => {
  const { status } = useSecuritySession();

  return (
    <div
      className={cn(
        'inline-flex items-center gap-2 rounded-full border px-3 py-2 text-[11px] font-semibold uppercase tracking-[0.18em]',
        statusClassName[status]
      )}
    >
      <span className={cn('h-2 w-2 rounded-full', statusDotClassName[status])} />
      {statusLabel[status]}
    </div>
  );
};

export { SecuritySessionProvider, SessionBadge, useSecuritySession };
