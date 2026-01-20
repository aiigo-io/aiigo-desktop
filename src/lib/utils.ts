import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

import { openUrl } from '@tauri-apps/plugin-opener';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function shortAddress(address: string) {
  return address.slice(0, 4) + '...' + address.slice(-4);
}

export const getEvmExplorerUrl = (txHash: string, chainId: number) => {
  const explorers: Record<number, string> = {
    1: 'https://etherscan.io/tx/',
    56: 'https://bscscan.com/tx/',
    137: 'https://polygonscan.com/tx/',
    42161: 'https://arbiscan.io/tx/',
    10: 'https://optimistic.etherscan.io/tx/',
    11155111: 'https://sepolia.etherscan.io/tx/'
  };
  const baseUrl = explorers[chainId] || explorers[1];
  return `${baseUrl}${txHash}`;
};

export const getBitcoinExplorerUrl = (txHash: string) => {
  return `https://blockstream.info/tx/${txHash}`;
};

export const openExternalLink = async (url: string) => {
  try {
    await openUrl(url);
  } catch (error) {
    console.error('Failed to open URL:', error);
  }
};
