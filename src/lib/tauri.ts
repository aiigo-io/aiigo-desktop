import { invoke as tauriInvoke } from '@tauri-apps/api/core';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke?: unknown;
    };
  }
}

export const TAURI_UNAVAILABLE_MESSAGE = 'Tauri runtime is unavailable. Start the app with `npm run tauri dev` for wallet features.';

export class TauriUnavailableError extends Error {
  constructor() {
    super(TAURI_UNAVAILABLE_MESSAGE);
    this.name = 'TauriUnavailableError';
  }
}

export function isTauriRuntimeAvailable(): boolean {
  return typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__?.invoke === 'function';
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntimeAvailable()) {
    throw new TauriUnavailableError();
  }

  return tauriInvoke<T>(command, args);
}

export function isTauriUnavailableError(error: unknown): boolean {
  return error instanceof TauriUnavailableError
    || (error instanceof Error && error.name === 'TauriUnavailableError')
    || String(error).includes(TAURI_UNAVAILABLE_MESSAGE);
}