export const TRANSACTION_STATE_EVENT = 'app-transaction-state-changed';

export function notifyTransactionStateChanged() {
  window.dispatchEvent(new Event(TRANSACTION_STATE_EVENT));
}