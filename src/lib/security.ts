import { invoke, isTauriRuntimeAvailable, TAURI_UNAVAILABLE_MESSAGE } from '@/lib/tauri';

export type SecurityError =
  | 'locked'
  | 'expired'
  | 'policy_denied'
  | 'operation_not_allowed'
  | 'unknown_wallet';

const SECURITY_ERRORS: SecurityError[] = [
  'locked',
  'expired',
  'policy_denied',
  'operation_not_allowed',
  'unknown_wallet',
];

export const SECURITY_STATE_EVENT = 'app-security-changed';
export const EXPORT_UNAVAILABLE_MESSAGE = 'Export is currently unavailable in this wallet MVP.';

export function isSecurityError(value: unknown): value is SecurityError {
  return typeof value === 'string' && SECURITY_ERRORS.includes(value as SecurityError);
}

export function parseSecurityError(error: unknown): SecurityError | null {
  if (isSecurityError(error)) {
    return error;
  }

  if (error && typeof error === 'object') {
    const maybeMessage = (error as { message?: unknown }).message;
    if (isSecurityError(maybeMessage)) {
      return maybeMessage;
    }
  }

  const fallback = String(error);
  return isSecurityError(fallback) ? fallback : null;
}

export function notifySecurityChanged() {
  window.dispatchEvent(new Event(SECURITY_STATE_EVENT));
}

async function invokeSecurityCommand<T>(command: string, args?: Record<string, unknown>) {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const securityError = parseSecurityError(error);
    if (securityError) {
      throw securityError;
    }

    throw error;
  }
}

export async function securityUnlock(token: string) {
  if (!isTauriRuntimeAvailable()) {
    throw new Error(TAURI_UNAVAILABLE_MESSAGE);
  }

  await invokeSecurityCommand<void>('security_unlock', { token });
}

export async function securityLock() {
  if (!isTauriRuntimeAvailable()) {
    return;
  }

  await invokeSecurityCommand<void>('security_lock');
}

export async function securityIsUnlocked() {
  if (!isTauriRuntimeAvailable()) {
    return false;
  }

  return invokeSecurityCommand<boolean>('security_is_unlocked');
}
