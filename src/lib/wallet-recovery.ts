import { parseSecurityError } from '@/lib/security';

export type WalletRecoveryFlow =
  | 'create-wallet'
  | 'import-wallet'
  | 'send-asset'
  | 'approve-allowance'
  | 'refresh-state';

type WalletRecoveryTone = 'error' | 'warning' | 'info';
type WalletRecoveryChainFamily = 'bitcoin' | 'evm' | 'generic';

export interface WalletRecoveryGuidance {
  title: string;
  summary: string;
  actions: string[];
  tone: WalletRecoveryTone;
  rawError: string;
}

interface WalletRecoveryOptions {
  chainFamily?: WalletRecoveryChainFamily;
}

const DEFAULT_RECOVERY_ACTIONS: Record<WalletRecoveryFlow, string[]> = {
  'create-wallet': [
    'Retry once after confirming the desktop wallet runtime is available.',
    'If the failure repeats, keep the current device state unchanged and inspect the raw error below.',
  ],
  'import-wallet': [
    'Re-check the secret you entered before retrying import.',
    'If this wallet may already exist locally, reuse the existing wallet entry instead of importing again.',
  ],
  'send-asset': [
    'Keep the reviewed recipient, amount, and fee visible, then retry only after the failure cause is resolved.',
    'Refresh balance or network state before attempting another broadcast.',
  ],
  'approve-allowance': [
    'Retry approval only after the active chain and spender still match the reviewed action.',
    'If allowance may have changed meanwhile, rebuild the approval review first.',
  ],
  'refresh-state': [
    'Retry refresh after confirming the selected network path is reachable.',
    'Treat current local state as cached until a later refresh succeeds.',
  ],
};

function normalizeError(error: unknown): string {
  if (typeof error === 'string') {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string') {
      return message;
    }
  }

  return String(error);
}

function buildNetworkMismatchActions(chainFamily: WalletRecoveryChainFamily): string[] {
  if (chainFamily === 'bitcoin') {
    return [
      'Use a recipient address that matches this wallet network instead of a mainnet/testnet mix.',
      'If you copied the address from another app, verify the prefix before retrying the send.',
    ];
  }

  if (chainFamily === 'evm') {
    return [
      'Switch back to the reviewed EVM chain before retrying this action.',
      'Rebuild the action review after any chain or token change so the target and gas values stay aligned.',
    ];
  }

  return [
    'Retry only after the reviewed network and the active runtime network match again.',
    'Rebuild the action review if any chain context changed after the last confirmation.',
  ];
}

export function describeWalletRecovery(
  flow: WalletRecoveryFlow,
  error: unknown,
  options: WalletRecoveryOptions = {},
): WalletRecoveryGuidance {
  const rawError = normalizeError(error);
  const normalized = rawError.toLowerCase();
  const chainFamily = options.chainFamily ?? 'generic';
  const securityError = parseSecurityError(error);

  if (securityError === 'locked') {
    return {
      title: 'Wallet is locked',
      summary: 'This action stopped before signing because the wallet session was not unlocked.',
      actions: [
        'Unlock the wallet again, then retry from the current screen.',
        'If this was a reviewed send or approval, confirm the payload again before broadcasting.',
      ],
      tone: 'warning',
      rawError,
    };
  }

  if (securityError === 'expired') {
    return {
      title: 'Session expired before completion',
      summary: 'The wallet session timed out before this high-risk action could finish signing.',
      actions: [
        'Unlock the wallet again and retry without leaving the review step open too long.',
        'If fees, gas, or chain context may have changed, rebuild the review before confirming again.',
      ],
      tone: 'warning',
      rawError,
    };
  }

  if (securityError === 'secret_backend_unavailable') {
    return {
      title: 'Secret backend is unavailable',
      summary: 'This action stopped before secret access because the local secure storage backend is currently unavailable.',
      actions: [
        'Restore local keyring or secret-service access before retrying create, import, sign, or export flows.',
        'Treat current balances and history as readable local state only until secure secret access recovers.',
      ],
      tone: 'warning',
      rawError,
    };
  }

  if (normalized.includes('address network mismatch') || normalized.includes('chain mismatch') || normalized.includes('network mismatch')) {
    return {
      title: 'Network mismatch',
      summary: 'The reviewed action no longer matches the network context needed to execute it safely.',
      actions: buildNetworkMismatchActions(chainFamily),
      tone: 'warning',
      rawError,
    };
  }

  if (normalized.includes('invalid mnemonic')) {
    return {
      title: 'Mnemonic format is invalid',
      summary: 'The wallet could not parse the mnemonic you entered as a valid recovery phrase.',
      actions: [
        'Enter the full 12- or 24-word phrase with single spaces between words.',
        'Remove trailing punctuation or line breaks copied from other apps before retrying import.',
      ],
      tone: 'error',
      rawError,
    };
  }

  if (normalized.includes('invalid private key')) {
    return {
      title: 'Private key format is invalid',
      summary: 'The wallet could not parse the private key you entered into a valid signing key.',
      actions: chainFamily === 'bitcoin'
        ? [
            'Provide either a valid WIF key or a 64-character hex private key.',
            'If the key came from a backup export, remove extra spaces before retrying import.',
          ]
        : [
            'Provide a 64-character hex private key, with or without the 0x prefix.',
            'If the key came from another wallet, confirm it belongs to the chain family you are importing.',
          ],
      tone: 'error',
      rawError,
    };
  }

  if (normalized.includes('already exists') || normalized.includes('unique constraint failed')) {
    return {
      title: 'Wallet is already present locally',
      summary: 'This device already has a wallet entry for the same imported secret or derived address.',
      actions: [
        'Reuse the existing wallet entry instead of importing the same secret again.',
        'If you only wanted a different display name, rename or relabel the existing wallet entry later.',
      ],
      tone: 'warning',
      rawError,
    };
  }

  if (normalized.includes('insufficient balance') || normalized.includes('not enough funds') || normalized.includes('insufficient funds')) {
    return {
      title: 'Insufficient balance for this action',
      summary: 'The wallet does not currently have enough spendable balance to complete the reviewed amount and fee.',
      actions: [
        'Reduce the amount or refresh wallet state before retrying.',
        'Keep network fees in mind, because the reviewed total must remain spendable at broadcast time.',
      ],
      tone: 'warning',
      rawError,
    };
  }

  if (normalized.includes('insufficient token allowance')) {
    return {
      title: 'Allowance no longer covers this swap',
      summary: 'The swap path needs a token approval that is no longer sufficient for the current amount.',
      actions: [
        'Run the approval step again from the current chain and token pair.',
        'If the swap inputs changed, rebuild the swap review before approving again.',
      ],
      tone: 'warning',
      rawError,
    };
  }

  if (normalized.includes('invalid address') || normalized.includes('invalid recipient')) {
    return {
      title: 'Recipient address is invalid',
      summary: 'The reviewed destination does not match a valid address format for this action.',
      actions: chainFamily === 'bitcoin'
        ? [
            'Paste a valid Bitcoin address for the same wallet network you are using.',
            'If you copied the address from chat or notes, check for missing characters before retrying.',
          ]
        : [
            'Paste a valid EVM address before rebuilding the review.',
            'If this target came from another chain, switch assets instead of forcing the current send path.',
          ],
      tone: 'error',
      rawError,
    };
  }

  if (normalized.includes('tauri') || normalized.includes('desktop wallet runtime unavailable')) {
    return {
      title: 'Desktop wallet runtime is unavailable',
      summary: 'This flow depends on the Tauri wallet runtime, but the current environment could not access it.',
      actions: [
        'Retry from the desktop wallet runtime instead of a plain browser context.',
        'If the desktop app is already open, restart the runtime and retry the same action.',
      ],
      tone: 'info',
      rawError,
    };
  }

  return {
    title: 'Action failed before completion',
    summary: 'The wallet stopped this flow before it reached a confirmed success state, so recovery should stay explicit.',
    actions: DEFAULT_RECOVERY_ACTIONS[flow],
    tone: 'error',
    rawError,
  };
}

export function getWalletRecoveryToneClass(tone: WalletRecoveryTone): string {
  switch (tone) {
    case 'warning':
      return 'border-amber-500/30 bg-amber-500/10 text-amber-800';
    case 'info':
      return 'border-sky-500/30 bg-sky-500/10 text-sky-800';
    case 'error':
      return 'border-red-500/30 bg-red-500/10 text-red-800';
  }
}