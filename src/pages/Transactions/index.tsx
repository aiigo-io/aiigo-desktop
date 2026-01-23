import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { ArrowUpRight, ArrowDownLeft, RefreshCw, Send, ExternalLink, ShieldCheck, Code } from 'lucide-react';
import { cn, shortAddress, getEvmExplorerUrl, getBitcoinExplorerUrl, openExternalLink } from '@/lib/utils';

interface BitcoinTransaction {
  id: string;
  wallet_id: string;
  tx_hash: string;
  tx_type: 'send' | 'receive';
  from_address: string;
  to_address: string;
  amount: number;
  fee: number;
  status: 'pending' | 'confirmed' | 'failed';
  confirmations: number;
  block_height: number | null;
  timestamp: string;
  created_at: string;
}

interface EvmTransaction {
  id: string;
  wallet_id: string;
  tx_hash: string;
  tx_type: 'send' | 'receive' | 'approve' | 'contract';
  from_address: string;
  to_address: string;
  amount: string;
  amount_float: number;
  asset_symbol: string;
  asset_name: string;
  contract_address: string | null;
  chain: string;
  chain_id: number;
  gas_used: string;
  gas_price: string;
  fee: number;
  status: 'pending' | 'confirmed' | 'failed';
  block_number: number | null;
  timestamp: string;
  created_at: string;
}

const Transactions: React.FC = () => {
  const [bitcoinTransactions, setBitcoinTransactions] = useState<BitcoinTransaction[]>([]);
  const [evmTransactions, setEvmTransactions] = useState<EvmTransaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [activeTab, setActiveTab] = useState('all');

  useEffect(() => {
    fetchTransactions();
  }, []);

  const fetchTransactions = async () => {
    setLoading(true);
    try {
      const [btcTxs, evmTxs] = await Promise.all([
        invoke<BitcoinTransaction[]>('get_all_bitcoin_transactions'),
        invoke<EvmTransaction[]>('get_all_evm_transactions'),
      ]);
      // Filter out failed transactions
      setBitcoinTransactions(btcTxs.filter(tx => tx.status !== 'failed'));
      setEvmTransactions(evmTxs.filter(tx => tx.status !== 'failed'));
    } catch (error) {
      console.error('Failed to fetch transactions:', error);
    } finally {
      setLoading(false);
    }
  };

  const syncTransactionsFromBlockchain = async () => {
    setSyncing(true);
    try {
      // Get all wallets
      const [btcWallets, evmWallets] = await Promise.all([
        invoke<any[]>('bitcoin_get_wallets'),
        invoke<any[]>('evm_get_wallets'),
      ]);

      // Sync Bitcoin transactions
      const btcSyncPromises = btcWallets.map(wallet =>
        invoke('fetch_bitcoin_history', {
          walletId: wallet.id,
          address: wallet.address,
        }).catch(err => {
          console.error(`Failed to sync Bitcoin wallet ${wallet.id}:`, err);
          return [];
        })
      );

      // Supported EVM chains
      const evmChains = [
        { name: 'Ethereum', chainId: 1 },
        { name: 'BSC', chainId: 56 },
        { name: 'Polygon', chainId: 137 },
        { name: 'Arbitrum', chainId: 42161 },
        { name: 'Optimism', chainId: 10 },
        { name: 'Sepolia', chainId: 11155111 }
      ];

      // Sync EVM transactions for all chains
      const evmSyncPromises = evmWallets.flatMap(wallet =>
        evmChains.map(chain =>
          invoke('fetch_evm_history', {
            walletId: wallet.id,
            address: wallet.address,
            chain: chain.name,
            chainId: chain.chainId,
          }).catch(err => {
            console.error(`Failed to sync ${chain.name} for wallet ${wallet.id}:`, err);
            return [];
          })
        )
      );

      await Promise.all([...btcSyncPromises, ...evmSyncPromises]);

      // Refresh transactions after sync
      await fetchTransactions();
    } catch (error) {
      console.error('Failed to sync transactions:', error);
    } finally {
      setSyncing(false);
    }
  };

  const formatDate = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'confirmed':
        return 'bg-green-500/10 text-green-500 border-green-500/20';
      case 'pending':
        return 'bg-yellow-500/10 text-yellow-500 border-yellow-500/20';
      case 'failed':
        return 'bg-red-500/10 text-red-500 border-red-500/20';
      default:
        return 'bg-gray-500/10 text-gray-500 border-gray-500/20';
    }
  };


  const BitcoinTransactionRow: React.FC<{ tx: BitcoinTransaction }> = ({ tx }) => {
    const isSend = tx.tx_type === 'send';

    return (
      <div className="flex items-center justify-between p-4 border-b border-border/50 hover:bg-muted/50 transition-colors">
        <div className="flex items-center gap-4 flex-1">
          <div className={cn(
            "p-2 rounded-full",
            isSend ? "bg-red-500/10" : "bg-green-500/10"
          )}>
            {isSend ? (
              <ArrowUpRight className="w-5 h-5 text-red-500" />
            ) : (
              <ArrowDownLeft className="w-5 h-5 text-green-500" />
            )}
          </div>

          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className="font-medium text-sm">
                {isSend ? 'Send Bitcoin' : 'Receive Bitcoin'}
              </span>
              <Badge variant="outline" className={cn("text-xs", getStatusColor(tx.status))}>
                {tx.status}
              </Badge>
            </div>
            <div className="text-xs text-muted-foreground space-y-1">
              <div>Hash: {shortAddress(tx.tx_hash)}</div>
              <div>{formatDate(tx.timestamp)}</div>
            </div>
          </div>

          <div className="text-right">
            <div className={cn(
              "font-semibold text-sm mb-1",
              isSend ? "text-red-500" : "text-green-500"
            )}>
              {isSend ? '-' : '+'}{tx.amount.toFixed(8)} BTC
            </div>
            <div className="text-xs text-muted-foreground">
              Fee: {tx.fee.toFixed(8)} BTC
            </div>
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => openExternalLink(getBitcoinExplorerUrl(tx.tx_hash))}
            className="ml-2"
          >
            <ExternalLink className="w-4 h-4" />
          </Button>
        </div>
      </div>
    );
  };

  const EvmTransactionRow: React.FC<{ tx: EvmTransaction }> = ({ tx }) => {
    const isSend = tx.tx_type === 'send';
    const isReceive = tx.tx_type === 'receive';
    const isApprove = tx.tx_type === 'approve';
    const isContract = tx.tx_type === 'contract';

    const getIcon = () => {
      if (isSend) return <ArrowUpRight className="w-5 h-5 text-red-500" />;
      if (isReceive) return <ArrowDownLeft className="w-5 h-5 text-green-500" />;
      if (isApprove) return <ShieldCheck className="w-5 h-5 text-blue-500" />;
      if (isContract) return <Code className="w-5 h-5 text-purple-500" />;
      return <Send className="w-5 h-5 text-muted-foreground" />;
    };

    const getIconBg = () => {
      if (isSend) return "bg-red-500/10";
      if (isReceive) return "bg-green-500/10";
      if (isApprove) return "bg-blue-500/10";
      if (isContract) return "bg-purple-500/10";
      return "bg-muted";
    };

    const getLabel = () => {
      if (isSend) return `Send ${tx.asset_symbol}`;
      if (isReceive) return `Receive ${tx.asset_symbol}`;
      if (isApprove) return `Approve ${tx.asset_symbol}`;
      if (isContract) return "Contract Interaction";
      return "Unknown Transaction";
    };

    const getAmountColor = () => {
      if (isSend || isApprove || isContract) return "text-red-500";
      if (isReceive) return "text-green-500";
      return "";
    };

    const getAmountPrefix = () => {
      if (isSend || isApprove || isContract) return "-";
      if (isReceive) return "+";
      return "";
    };

    return (
      <div className="flex items-center justify-between p-4 border-b border-border/50 hover:bg-muted/50 transition-colors">
        <div className="flex items-center gap-4 flex-1">
          <div className={cn("p-2 rounded-full", getIconBg())}>
            {getIcon()}
          </div>

          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className="font-medium text-sm">
                {getLabel()}
              </span>
              <Badge variant="outline" className="text-xs">
                {tx.chain}
              </Badge>
              <Badge variant="outline" className={cn("text-xs", getStatusColor(tx.status))}>
                {tx.status}
              </Badge>
            </div>
            <div className="text-xs text-muted-foreground space-y-1">
              <div>Hash: {shortAddress(tx.tx_hash)}</div>
              <div>{formatDate(tx.timestamp)}</div>
            </div>
          </div>

          <div className="text-right">
            <div className={cn("font-semibold text-sm mb-1", getAmountColor())}>
              {getAmountPrefix()}{tx.amount_float.toFixed(6)} {tx.asset_symbol}
            </div>
            <div className="text-xs text-muted-foreground">
              Fee: {tx.fee.toFixed(6)} {tx.chain === 'BSC' ? 'BNB' : (tx.chain === 'Polygon' ? 'POL' : 'ETH')}
            </div>
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => openExternalLink(getEvmExplorerUrl(tx.tx_hash, tx.chain_id))}
            className="ml-2"
          >
            <ExternalLink className="w-4 h-4" />
          </Button>
        </div>
      </div>
    );
  };

  const allTransactions = [
    ...bitcoinTransactions.map(tx => ({ ...tx, type: 'bitcoin' as const })),
    ...evmTransactions.map(tx => ({ ...tx, type: 'evm' as const })),
  ].sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Transactions</h1>
          <p className="text-muted-foreground mt-1">View and manage your transaction history</p>
        </div>
        <div className="flex gap-2">
          <Button onClick={syncTransactionsFromBlockchain} disabled={syncing} variant="outline" size="sm">
            <RefreshCw className={cn("w-4 h-4 mr-2", syncing && "animate-spin")} />
            Sync from Blockchain
          </Button>
          <Button onClick={fetchTransactions} disabled={loading} variant="outline" size="sm">
            <RefreshCw className={cn("w-4 h-4 mr-2", loading && "animate-spin")} />
            Refresh
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Transaction History</CardTitle>
          <CardDescription>
            All your Bitcoin and EVM transactions in one place
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Tabs value={activeTab} onValueChange={setActiveTab}>
            <TabsList className="grid w-full grid-cols-3 mb-6">
              <TabsTrigger value="all">
                All ({allTransactions.length})
              </TabsTrigger>
              <TabsTrigger value="bitcoin">
                Bitcoin ({bitcoinTransactions.length})
              </TabsTrigger>
              <TabsTrigger value="evm">
                EVM ({evmTransactions.length})
              </TabsTrigger>
            </TabsList>

            <TabsContent value="all" className="space-y-0">
              {loading ? (
                <div className="text-center py-12 text-muted-foreground">
                  Loading transactions...
                </div>
              ) : allTransactions.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Send className="w-12 h-12 mx-auto mb-4 opacity-20" />
                  <p>No transactions yet</p>
                  <p className="text-sm mt-2">Your transactions will appear here once you send or receive crypto</p>
                </div>
              ) : (
                <div className="border border-border/50 rounded-lg overflow-hidden">
                  {allTransactions.map((tx) => (
                    tx.type === 'bitcoin' ? (
                      <BitcoinTransactionRow key={tx.id} tx={tx as BitcoinTransaction} />
                    ) : (
                      <EvmTransactionRow key={tx.id} tx={tx as EvmTransaction} />
                    )
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="bitcoin" className="space-y-0">
              {loading ? (
                <div className="text-center py-12 text-muted-foreground">
                  Loading Bitcoin transactions...
                </div>
              ) : bitcoinTransactions.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Send className="w-12 h-12 mx-auto mb-4 opacity-20" />
                  <p>No Bitcoin transactions yet</p>
                </div>
              ) : (
                <div className="border border-border/50 rounded-lg overflow-hidden">
                  {bitcoinTransactions.map((tx) => (
                    <BitcoinTransactionRow key={tx.id} tx={tx} />
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="evm" className="space-y-0">
              {loading ? (
                <div className="text-center py-12 text-muted-foreground">
                  Loading EVM transactions...
                </div>
              ) : evmTransactions.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Send className="w-12 h-12 mx-auto mb-4 opacity-20" />
                  <p>No EVM transactions yet</p>
                </div>
              ) : (
                <div className="border border-border/50 rounded-lg overflow-hidden">
                  {evmTransactions.map((tx) => (
                    <EvmTransactionRow key={tx.id} tx={tx} />
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
};

export default Transactions;
