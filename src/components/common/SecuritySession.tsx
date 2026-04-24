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
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  LOCAL_PASSWORD_DEVICE_MESSAGE,
  LOCAL_PASSWORD_RECOVERY_MESSAGE,
  LOCAL_PASSWORD_SYNC_MESSAGE,
  SECURITY_STATE_EVENT,
  getBackendUnavailableReason,
  notifySecurityChanged,
  parseSecurityError,
  securityAuthorizeOperation,
  securityGetBackendState,
  securityGetLocalPasswordPolicy,
  securityHasPassword,
  securityIsUnlocked,
  securityLock,
  securityProbeBackend,
  securityResetLocalWalletData,
  securitySetupPassword,
  securityUnlock,
  type LocalPasswordPolicy,
  type SecurityBackendState,
  type SignerOperation,
} from '@/lib/security';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

type GateStatus = 'loading' | 'locked' | 'unlocked';
type UnlockReason = 'locked' | 'expired' | 'reauth_required' | 'setup_required';
type UnlockMode = 'unlock' | 'setup' | 'reauth';

interface PendingAction {
  prompt?: string;
  onUnlockSuccess?: () => void | Promise<void>;
  reason: UnlockReason;
  mode: UnlockMode;
  operation?: SignerOperation;
  resolve?: (completed: boolean) => void;
}

interface UnlockRequest {
  prompt?: string;
  onUnlockSuccess?: () => void | Promise<void>;
  reason?: UnlockReason;
  mode?: UnlockMode;
  operation?: SignerOperation;
}

interface SecuritySessionContextValue {
  status: GateStatus;
  backendState: SecurityBackendState | null;
  policy: LocalPasswordPolicy | null;
  requestUnlock: (action?: UnlockRequest) => Promise<boolean>;
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
  const [password, setPassword] = useState('');
  const [hasPassword, setHasPassword] = useState<boolean | null>(null);
  const [backendState, setBackendState] = useState<SecurityBackendState | null>(null);
  const [policy, setPolicy] = useState<LocalPasswordPolicy | null>(null);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [isUnlocking, setIsUnlocking] = useState(false);
  const [isResetDialogOpen, setIsResetDialogOpen] = useState(false);
  const [isResetting, setIsResetting] = useState(false);
  const actionRef = useRef<PendingAction | null>(null);
  const statusRef = useRef<GateStatus>('loading');

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const handleSecurityError = useCallback((error: unknown) => {
    const securityError = parseSecurityError(error);

    switch (securityError) {
      case 'no_password':
        setHasPassword(false);
        setInlineError('Set a local password before unlocking signing access.');
        break;
      case 'wrong_password':
        setInlineError('Wrong password. Try again.');
        break;
      case 'reauth_required':
        setInlineError('Re-enter your local password for this high-risk action.');
        break;
      case 'policy_denied':
        setInlineError(hasPassword === false ? 'Enter a non-empty password to set it.' : 'Enter a non-empty password.');
        break;
      case 'operation_not_allowed':
        toast.error('Unlock is temporarily unavailable. Try again.');
        break;
      case 'secret_backend_unavailable':
        toast.error('Secret backend unavailable. Signing and export remain disabled.');
        break;
      case 'unknown_wallet':
        toast.error('Wallet was not found. Refresh and try again.');
        break;
      case 'expired':
      case 'locked':
        setStatus('locked');
        break;
      default:
        toast.error(String(error));
    }

    return securityError;
  }, [hasPassword]);

  const refreshStatus = useCallback(async () => {
    try {
      const [unlocked, passwordConfigured, nextBackendState, nextPolicy] = await Promise.all([
        securityIsUnlocked(),
        securityHasPassword(),
        securityGetBackendState(),
        securityGetLocalPasswordPolicy(),
      ]);
      setStatus(unlocked ? 'unlocked' : 'locked');
      setHasPassword(passwordConfigured);
      setBackendState(nextBackendState);
      setPolicy(nextPolicy);
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
    const openUnlockDialog = (requestedMode: UnlockMode, resolve: (completed: boolean) => void) => {
      const nextAction: PendingAction = {
        prompt: action.prompt,
        onUnlockSuccess: action.onUnlockSuccess,
        reason: action.reason ?? (requestedMode === 'reauth' ? 'reauth_required' : 'locked'),
        mode: requestedMode,
        operation: action.operation,
        resolve,
      };

      actionRef.current = nextAction;
      setPendingAction(nextAction);
      setPassword('');
      setInlineError(null);
      setIsDialogOpen(true);
    };

    try {
      const requestedMode = action.mode ?? 'unlock';

      if (requestedMode === 'setup') {
        const passwordConfigured = hasPassword ?? await securityHasPassword();
        setHasPassword(passwordConfigured);
        if (passwordConfigured) {
          await action.onUnlockSuccess?.();
          return true;
        }
      }

      if (requestedMode === 'reauth') {
        return await new Promise<boolean>((resolve) => {
          openUnlockDialog('reauth', resolve);
        });
      }

      const unlocked = status === 'unlocked' ? true : await refreshStatus();
      if (unlocked) {
        await action.onUnlockSuccess?.();
        return true;
      }

      return await new Promise<boolean>((resolve) => {
        openUnlockDialog(requestedMode, resolve);
      });
    } catch (error) {
      handleSecurityError(error);
      return false;
    }
  }, [refreshStatus, status, hasPassword, handleSecurityError]);

  const handleUnlock = useCallback(async () => {
    setIsUnlocking(true);
    setInlineError(null);

    try {
      const nextAction = actionRef.current;
      const mode = nextAction?.mode ?? 'unlock';
      const operation = nextAction?.operation;

      if (hasPassword === false) {
        await securitySetupPassword(password);
        setHasPassword(true);
      }

      if (mode === 'reauth') {
        if (!operation) {
          throw new Error('High-risk action requires an operation context.');
        }

        await securityAuthorizeOperation(password, operation);
      } else if (mode === 'unlock') {
        await securityUnlock(password);
      }

      const nextStatus = mode === 'setup'
        ? (await securityIsUnlocked() ? 'unlocked' : 'locked')
        : 'unlocked';

      setStatus(nextStatus);
      setBackendState(await securityProbeBackend());
      setIsDialogOpen(false);
      setPassword('');
      notifySecurityChanged();

      actionRef.current = null;
      setPendingAction(null);
      nextAction?.resolve?.(true);

      if (nextAction?.onUnlockSuccess) {
        await nextAction.onUnlockSuccess();
      }
    } catch (error) {
      handleSecurityError(error);
    } finally {
      setIsUnlocking(false);
    }
  }, [handleSecurityError, hasPassword, password]);

  const handleOpenChange = useCallback((open: boolean) => {
    setIsDialogOpen(open);
    if (!open) {
      actionRef.current?.resolve?.(false);
      actionRef.current = null;
      setPendingAction(null);
      setPassword('');
      setInlineError(null);
    }
  }, []);

  const contextValue = useMemo<SecuritySessionContextValue>(() => ({
    status,
    backendState,
    policy,
    requestUnlock,
    refreshStatus,
  }), [backendState, policy, refreshStatus, requestUnlock, status]);

  const dialogMode = pendingAction?.mode ?? 'unlock';
  const isPasswordSetup = dialogMode === 'setup' || hasPassword === false;
  const dialogTitle = dialogMode === 'reauth'
    ? 'Password Re-Auth Required'
    : isPasswordSetup
      ? 'Set Local Password'
      : pendingAction?.reason === 'expired'
        ? 'Session expired'
        : 'Unlock wallet';
  const dialogDescription = dialogMode === 'reauth'
    ? 'High-risk actions require you to re-enter the local password even when the wallet is already unlocked.'
    : isPasswordSetup
      ? 'Create the per-installation local password before create, import, send, approve, reveal, or export continue.'
      : pendingAction?.reason === 'expired'
        ? 'Unlock again to continue the same action.'
        : 'Use the local password from this device to continue.';
  const backendUnavailableReason = getBackendUnavailableReason(backendState);

  const handleResetLocalWalletData = useCallback(async () => {
    setIsResetting(true);
    try {
      await securityResetLocalWalletData();
      notifySecurityChanged();
      setIsResetDialogOpen(false);
      setIsDialogOpen(false);
      setPendingAction(null);
      actionRef.current = null;
      toast.success('Local wallet data was reset. Restore wallets with your recovery phrase or private key.');
      window.setTimeout(() => {
        window.location.reload();
      }, 200);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsResetting(false);
    }
  }, []);

  const lockSessionForPolicy = useCallback(async (reason: 'idle' | 'sleep') => {
    if (statusRef.current !== 'unlocked') {
      return;
    }

    try {
      await securityLock();
      setStatus('locked');
      setInlineError(
        reason === 'idle'
          ? 'Session locked after 15 minutes of inactivity.'
          : 'Session locked after system sleep.'
      );
      notifySecurityChanged();
    } catch (error) {
      toast.error(String(error));
    }
  }, []);

  useEffect(() => {
    if (status !== 'unlocked') {
      return;
    }

    const idleLockMs = (policy?.idle_lock_seconds ?? 15 * 60) * 1000;
    const sleepLockEnabled = policy?.lock_on_system_sleep ?? true;
    const sleepCheckIntervalMs = 1000;
    const sleepGapGraceMs = 2000;
    let idleTimeout: number | null = null;
    let sleepCheckInterval: number | null = null;
    let lastSleepCheckAt = Date.now();

    const scheduleIdleLock = () => {
      if (idleTimeout !== null) {
        window.clearTimeout(idleTimeout);
      }

      idleTimeout = window.setTimeout(() => {
        void lockSessionForPolicy('idle');
      }, idleLockMs);
    };

    const handleActivity = () => {
      scheduleIdleLock();
    };

    const detectSleepGap = (now = Date.now()) => {
      if (!sleepLockEnabled) {
        return false;
      }

      const delayedByMs = now - lastSleepCheckAt;
      lastSleepCheckAt = now;

      if (delayedByMs > sleepCheckIntervalMs + sleepGapGraceMs) {
        void lockSessionForPolicy('sleep');
        return true;
      }

      return false;
    };

    const handleVisibilityChange = () => {
      if (!sleepLockEnabled || document.visibilityState !== 'visible') {
        return;
      }

      if (detectSleepGap()) {
        return;
      }

      scheduleIdleLock();
    };

    const handleWindowFocus = () => {
      if (detectSleepGap()) {
        return;
      }

      scheduleIdleLock();
    };

    const trackedEvents: Array<keyof WindowEventMap> = ['mousemove', 'mousedown', 'keydown', 'scroll', 'touchstart', 'focus'];
    for (const eventName of trackedEvents) {
      window.addEventListener(eventName, handleActivity, { passive: true });
    }

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleWindowFocus);
    scheduleIdleLock();
    sleepCheckInterval = window.setInterval(() => {
      detectSleepGap();
    }, sleepCheckIntervalMs);

    return () => {
      if (idleTimeout !== null) {
        window.clearTimeout(idleTimeout);
      }
      if (sleepCheckInterval !== null) {
        window.clearInterval(sleepCheckInterval);
      }
      for (const eventName of trackedEvents) {
        window.removeEventListener(eventName, handleActivity);
      }
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleWindowFocus);
    };
  }, [lockSessionForPolicy, policy, status]);

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
                {dialogMode === 'reauth' ? 'Re-Auth Required' : 'Locked'}
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
            <div className="rounded-xl border border-cyan-400/20 bg-cyan-400/5 px-4 py-3 text-sm text-cyan-50/90">
              <p>{LOCAL_PASSWORD_DEVICE_MESSAGE}</p>
              <p className="mt-1">{LOCAL_PASSWORD_RECOVERY_MESSAGE}</p>
              <p className="mt-1">{LOCAL_PASSWORD_SYNC_MESSAGE}</p>
              {policy && (
                <p className="mt-1 text-cyan-100/80">
                  Session auto-locks after {Math.round(policy.idle_lock_seconds / 60)} minutes of inactivity{policy.lock_on_system_sleep ? ' and after system sleep' : ''}. Fresh password re-auth stays valid for up to {policy.reauth_window_seconds} seconds and is consumed by one high-risk action.
                </p>
              )}
            </div>

            {backendState?.degraded && (
              <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
                <p className="font-semibold uppercase tracking-[0.18em] text-[11px] text-amber-200">Degraded Security</p>
                <p className="mt-2 leading-relaxed text-amber-50/90">
                  {backendUnavailableReason
                    ? `Secure secret storage is unavailable: ${backendUnavailableReason.message}`
                    : 'Legacy plaintext secrets still need migration or secure secret access is not fully healthy.'}
                </p>
              </div>
            )}

            <div className="space-y-2">
              <label className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">
                {isPasswordSetup ? 'Password Setup' : 'Password'}
              </label>
              <Input
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                placeholder={isPasswordSetup ? 'Create a local password' : 'Enter password'}
                className="h-14 border-border/70 bg-[#121924] text-lg text-white placeholder:text-slate-500"
              />
              {inlineError && (
                <p className="text-sm text-destructive">{inlineError}</p>
              )}
            </div>

            {hasPassword !== false && (
              <button
                type="button"
                className="text-left text-sm text-amber-200 underline underline-offset-4"
                onClick={() => setIsResetDialogOpen(true)}
              >
                Forgot local password?
              </button>
            )}

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
                {isUnlocking
                  ? (dialogMode === 'reauth' ? 'Authorizing...' : isPasswordSetup ? 'Saving...' : 'Unlocking...')
                  : (dialogMode === 'reauth' ? 'Authorize Action' : isPasswordSetup ? 'Set Password' : 'Unlock')}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <AlertDialog open={isResetDialogOpen} onOpenChange={setIsResetDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Reset Local Wallet Data</AlertDialogTitle>
            <AlertDialogDescription>
              We cannot recover or reveal the old local password. The only supported path is to delete local wallet data on this device and restore again with your recovery phrase or private key.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950">
            This reset is destructive for local wallet data only. It does not reset funds on-chain, and it does not replace your recovery material.
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isResetting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-red-600 text-white hover:bg-red-500"
              onClick={(event) => {
                event.preventDefault();
                void handleResetLocalWalletData();
              }}
              disabled={isResetting}
            >
              {isResetting ? 'Resetting...' : 'Reset Local Wallet Data'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
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
