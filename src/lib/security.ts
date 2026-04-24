import { invoke, isTauriRuntimeAvailable, TAURI_UNAVAILABLE_MESSAGE } from '@/lib/tauri';

export type SecurityError =
  | 'locked'
  | 'expired'
  | 'no_password'
  | 'wrong_password'
  | 'reauth_required'
  | 'policy_denied'
  | 'operation_not_allowed'
  | 'unknown_wallet'
  | 'secret_backend_unavailable';

export type SignerOperation = 'send' | 'approve' | 'export_mnemonic' | 'export_private_key';

export interface LocalPasswordPolicy {
  installation_scope: 'per_installation';
  device_only: boolean;
  cloud_sync: boolean;
  replaces_recovery_phrase: boolean;
  requires_password_before_create_or_import: boolean;
  requires_backend_ready_for_create_import_and_high_risk: boolean;
  idle_lock_seconds: number;
  lock_on_system_sleep: boolean;
  high_risk_reauth_operations: SignerOperation[];
  forgot_password_mode: 'reset_local_data_and_restore_with_recovery_material';
  reauth_window_seconds: number;
}

export interface SecretBackendUnavailableReason {
  kind: 'keyring_unavailable' | 'secret_service_unreachable' | 'key_decode_failed' | 'access_denied' | 'unknown_backend_error';
  message: string;
}

export type SecretBackendStatus =
  | 'ready'
  | 'unknown'
  | { unavailable: { reason: SecretBackendUnavailableReason } };

export interface SecretMigrationState {
  attempted_rows: number;
  migrated_rows: number;
  skipped_rows: number;
  failed_rows: number;
}

export interface SecurityBackendState {
  backend_status: SecretBackendStatus;
  migration: SecretMigrationState;
  has_legacy_plaintext_secrets: boolean;
  degraded: boolean;
}

const SECURITY_ERRORS: SecurityError[] = [
  'locked',
  'expired',
  'no_password',
  'wrong_password',
  'reauth_required',
  'policy_denied',
  'operation_not_allowed',
  'unknown_wallet',
  'secret_backend_unavailable',
];

export const SECURITY_STATE_EVENT = 'app-security-changed';
export const EXPORT_UNAVAILABLE_MESSAGE = 'Export is currently unavailable in this wallet MVP.';
export const LOCAL_PASSWORD_DEVICE_MESSAGE = 'Local password only protects this device.';
export const LOCAL_PASSWORD_RECOVERY_MESSAGE = 'It does not replace your recovery phrase or private key.';
export const LOCAL_PASSWORD_SYNC_MESSAGE = 'It does not sync to cloud.';

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

export function getBackendUnavailableReason(state: SecurityBackendState | null | undefined) {
  if (!state) {
    return null;
  }

  return typeof state.backend_status === 'object' && 'unavailable' in state.backend_status
    ? state.backend_status.unavailable.reason
    : null;
}

export async function securityHasPassword() {
  if (!isTauriRuntimeAvailable()) {
    return false;
  }

  return invokeSecurityCommand<boolean>('security_has_password');
}

export async function securitySetupPassword(password: string) {
  if (!isTauriRuntimeAvailable()) {
    throw new Error(TAURI_UNAVAILABLE_MESSAGE);
  }

  await invokeSecurityCommand<void>('security_setup_password', { password });
}

export async function securityUnlock(password: string) {
  if (!isTauriRuntimeAvailable()) {
    throw new Error(TAURI_UNAVAILABLE_MESSAGE);
  }

  await invokeSecurityCommand<void>('security_unlock', { password });
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

export async function securityGetBackendState() {
  if (!isTauriRuntimeAvailable()) {
    return null;
  }

  return invokeSecurityCommand<SecurityBackendState>('security_get_backend_state');
}

export async function securityProbeBackend() {
  if (!isTauriRuntimeAvailable()) {
    return null;
  }

  return invokeSecurityCommand<SecurityBackendState>('security_probe_backend');
}

export async function securityGetLocalPasswordPolicy() {
  if (!isTauriRuntimeAvailable()) {
    return null;
  }

  return invokeSecurityCommand<LocalPasswordPolicy>('security_get_local_password_policy');
}

export async function securityAuthorizeOperation(password: string, operation: SignerOperation) {
  if (!isTauriRuntimeAvailable()) {
    throw new Error(TAURI_UNAVAILABLE_MESSAGE);
  }

  await invokeSecurityCommand<void>('security_authorize_operation', { password, operation });
}

export async function securityResetLocalWalletData() {
  if (!isTauriRuntimeAvailable()) {
    throw new Error(TAURI_UNAVAILABLE_MESSAGE);
  }

  await invokeSecurityCommand<void>('security_reset_local_wallet_data');
}
