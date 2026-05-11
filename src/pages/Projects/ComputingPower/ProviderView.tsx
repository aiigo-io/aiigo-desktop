import React, { useState } from 'react';

import { useSecuritySession } from '@/components/common/SecuritySession';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { Activity, CheckCircle2, Server, Wallet } from 'lucide-react';
import { toast } from 'sonner';

import { cn } from '@/lib/utils';
import type { ComputeMarketplaceModel, ResourceType } from './useComputeMarketplace';

const ProviderView: React.FC<{ model: ComputeMarketplaceModel }> = ({ model }) => {
    const { requestUnlock } = useSecuritySession();
    const { selectedWallet, config, snapshot, registerNode, verifyNode, acceptTask, submitResult } = model;

    const [showRegistration, setShowRegistration] = useState(false);
    const [regType, setRegType] = useState<ResourceType>('GPU');
    const [regModel, setRegModel] = useState('');
    const [regRegion, setRegRegion] = useState('US-East');
    const [regComputePower, setRegComputePower] = useState(1000);
    const [stakeAmount, setStakeAmount] = useState(0.5);
    const [resultUris, setResultUris] = useState<Record<string, string>>({});

    const myNodes = snapshot.ownedNodes;
    const primaryNode = myNodes[0] ?? null;
    const availableTasks = snapshot.openTasks.filter((task) => {
        if (!primaryNode) {
            return false;
        }

        return task.resourceType === primaryNode.resourceType
            && task.minTrustLevel <= primaryNode.trustLevel
            && task.requiredPower <= primaryNode.computePower;
    });
    const myActiveTasks = snapshot.providerTasks.filter((task) => task.status === 'Assigned' || task.status === 'InProgress');

    const authorizeSend = async (prompt: string) => requestUnlock({ operation: 'send', prompt });

    const handleRegister = async () => {
        try {
            if (!(await authorizeSend('Authorize compute node registration'))) {
                return;
            }

            await registerNode({
                resourceType: regType,
                computePower: String(regComputePower),
                gpuModel: regModel,
                region: regRegion,
                stakeEth: stakeAmount.toFixed(1),
            });
            setShowRegistration(false);
            setRegModel('');
            toast.success('Node registration submitted (broadcasting). Activation requires on-chain confirmation — refresh chain state to verify.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    const handleVerifyNode = async (nodeId: string) => {
        try {
            if (!(await authorizeSend('Authorize PoW activation challenge'))) {
                return;
            }
            await verifyNode(nodeId);
            toast.success('Activation submitted. Refresh chain state to confirm Active status.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    const handleAcceptTask = async (taskId: string) => {        try {
            if (!primaryNode) {
                throw new Error('Register and refresh a provider node before accepting tasks.');
            }
            if (!(await authorizeSend('Authorize task acceptance'))) {
                return;
            }

            await acceptTask(taskId, primaryNode.nodeId);
            toast.success('Task acceptance submitted (broadcasting). Refresh chain state to confirm assignment.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    const handleSubmitResult = async (taskId: string) => {
        try {
            const resultUri = resultUris[taskId]?.trim();
            if (!resultUri) {
                throw new Error('Enter a result URI before submitting.');
            }
            if (!(await authorizeSend('Authorize task result submission'))) {
                return;
            }

            await submitResult(taskId, resultUri);
            toast.success('Result submitted (broadcasting). Refresh chain state to confirm completion.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    if (showRegistration) {
        return (
            <div className="max-w-3xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
                <div className="flex items-center gap-4">
                    <Button variant="ghost" size="sm" onClick={() => setShowRegistration(false)}>
                        ← Back to Dashboard
                    </Button>
                </div>
                <div className="text-center space-y-4">
                    <h2 className="text-3xl font-black tracking-tight">Register Your Node</h2>
                    <p className="text-muted-foreground">This form submits a real payable NodeRegistry transaction. Node activation remains an on-chain verification outcome, not a local toggle.</p>
                </div>

                <Card className="bg-white/[0.02] border-white/[0.05] py-6">
                    <CardHeader>
                        <CardTitle>Node Configuration</CardTitle>
                        <CardDescription>Current flow: registration fee + stake now, activation after the separate verification path.</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-6">
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label>Resource Type</Label>
                                <Select value={regType} onValueChange={(value: ResourceType) => setRegType(value)}>
                                    <SelectTrigger>
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="GPU">GPU</SelectItem>
                                        <SelectItem value="CPU">CPU</SelectItem>
                                        <SelectItem value="Network">Network</SelectItem>
                                        <SelectItem value="Mobile">Mobile</SelectItem>
                                        <SelectItem value="IoT">IoT</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                            <div className="space-y-2">
                                <Label>Region</Label>
                                <Select value={regRegion} onValueChange={setRegRegion}>
                                    <SelectTrigger>
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="US-East">US East</SelectItem>
                                        <SelectItem value="EU-Central">EU Central</SelectItem>
                                        <SelectItem value="Asia-East">Asia East</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                        </div>

                        <div className="space-y-2">
                            <Label>Hardware Model</Label>
                            <Input placeholder="e.g. RTX 4090, A100 80GB" value={regModel} onChange={(event) => setRegModel(event.target.value)} />
                        </div>

                        <div className="space-y-2">
                            <Label>Compute Power (TFLOPS)</Label>
                            <Input
                                type="number"
                                min={1}
                                placeholder="e.g. 1000"
                                value={regComputePower}
                                onChange={(event) => setRegComputePower(Number(event.target.value))}
                            />
                            <p className="text-xs text-muted-foreground">Tasks require a minimum compute power to be matched to your node.</p>
                        </div>

                        <div className="space-y-4 pt-4 border-t border-white/5">
                            <div className="flex justify-between">
                                <Label>ETH Stake Amount</Label>
                                <span className="font-mono font-bold text-primary">{stakeAmount.toFixed(1)} ETH</span>
                            </div>
                            <Slider value={[stakeAmount]} onValueChange={(value) => setStakeAmount(value[0])} min={0.5} max={10} step={0.1} className="[&_.bg-primary]:bg-primary" />
                            <div className="text-xs text-muted-foreground flex justify-between">
                                <span>Registration fee: 0.1 ETH</span>
                                <span className={cn(stakeAmount >= 5 ? 'text-emerald-400' : stakeAmount >= 3 ? 'text-blue-400' : 'text-yellow-400')}>
                                    Estimated trust band: {stakeAmount >= 5 ? 'Partner' : stakeAmount >= 3 ? 'Trusted' : stakeAmount >= 1 ? 'Verified' : 'Basic'}
                                </span>
                            </div>
                        </div>

                        <Button className="w-full h-12 font-bold text-base mt-2" onClick={() => void handleRegister()} disabled={!config.isConfigured || !selectedWallet || !regModel.trim()}>
                            <Wallet className="w-5 h-5 mr-2" />
                            Pay {(stakeAmount + 0.1).toFixed(1)} ETH & Register
                        </Button>
                    </CardContent>
                </Card>
            </div>
        );
    }

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Card className="bg-white/[0.02] border-white/[0.05] py-5">
                    <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Node Status</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="flex items-center justify-between">
                            <div className={cn('text-2xl font-black', primaryNode ? (primaryNode.status === 'Active' ? 'text-emerald-400' : 'text-yellow-400') : 'text-muted-foreground')}>
                                {primaryNode ? primaryNode.status : 'N/A'}
                            </div>
                            <Badge variant="outline" className="border-white/10 text-muted-foreground">{config.chainName}</Badge>
                        </div>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05] py-5">
                    <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Trust Level</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-blue-400">{primaryNode ? `Level ${primaryNode.trustLevel}` : '-'}</div>
                        <p className="text-xs text-muted-foreground mt-1">Reputation: {primaryNode ? primaryNode.reputation : '-'}</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05] py-5">
                    <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Pending Provider Claim</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-primary">{snapshot.pendingProviderPayoutEth.toFixed(4)} ETH</div>
                        <p className="text-xs text-muted-foreground mt-1">Claimable payout tracked by EscrowManager.</p>
                    </CardContent>
                </Card>
            </div>

            {/* Activation prompt — shown when node is Pending or Verified (not yet Active) */}
            {primaryNode && (primaryNode.status === 'Pending' || primaryNode.status === 'Verified') && (
                <Card className="border-yellow-500/30 bg-yellow-500/5 py-4">
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2 text-yellow-400">
                            <Server className="w-5 h-5" />
                            Node Activation Required
                        </CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-3">
                        <p className="text-sm text-muted-foreground">
                            Your node is <span className="font-semibold text-yellow-400">{primaryNode.status}</span>.
                            Complete the on-chain PoW challenge to become <span className="font-semibold text-emerald-400">Active</span> and start accepting tasks.
                        </p>
                        <Button
                            variant="outline"
                            className="border-yellow-500/40 text-yellow-400 hover:bg-yellow-500/10"
                            onClick={() => void handleVerifyNode(primaryNode.nodeId)}
                            disabled={!config.isConfigured || !selectedWallet}
                        >
                            <CheckCircle2 className="w-4 h-4 mr-2" />
                            Activate Node (PoW Challenge)
                        </Button>
                        <p className="text-xs text-muted-foreground">
                            Requires <code>AIIGO_COMPUTE_POW_VERIFIER_ADDRESS</code> to be set. The challenge is solved locally and submitted on-chain.
                        </p>
                    </CardContent>
                </Card>
            )}

            {myActiveTasks.length > 0 && (
                <Card className="border-primary/50 bg-primary/5 py-4">
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2">
                            <Activity className="w-5 h-5 text-primary animate-pulse" />
                            Running Task: {myActiveTasks[0].specTitle}
                        </CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="flex justify-between items-center gap-4">
                            <div className="space-y-1">
                                <p className="text-sm text-muted-foreground">Expected Duration: {myActiveTasks[0].durationSeconds / 3600}h</p>
                                <p className="text-sm text-muted-foreground">Gross escrow: {myActiveTasks[0].escrowAmountEth.toFixed(4)} ETH</p>
                            </div>
                            <Badge variant="outline" className="border-primary/20 text-primary">{myActiveTasks[0].status}</Badge>
                        </div>
                        <div className="grid gap-3 md:grid-cols-[1fr_auto]">
                            <Input
                                placeholder="ipfs://... or result artifact URI"
                                value={resultUris[myActiveTasks[0].taskId] ?? ''}
                                onChange={(event) => setResultUris((current) => ({ ...current, [myActiveTasks[0].taskId]: event.target.value }))}
                            />
                            <Button onClick={() => void handleSubmitResult(myActiveTasks[0].taskId)}>
                                <CheckCircle2 className="w-4 h-4 mr-2" />
                                Submit Result
                            </Button>
                        </div>
                    </CardContent>
                </Card>
            )}

            <div className="space-y-4">
                <div className="flex items-center justify-between">
                    <h3 className="text-xl font-bold tracking-tight">My Nodes</h3>
                    <Button variant="outline" size="sm" onClick={() => setShowRegistration(true)}>
                        <Server className="w-4 h-4 mr-2" />
                        Register New Node
                    </Button>
                </div>
                <div className="grid gap-4">
                    {myNodes.length === 0 ? (
                        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-6 text-sm text-muted-foreground">
                            No nodes were found for the selected wallet in the current chain snapshot.
                        </div>
                    ) : myNodes.map((node) => (
                        <Card key={node.nodeId} className="bg-white/[0.02] border-white/[0.05] py-4">
                            <CardContent className="flex items-center justify-between">
                                <div className="flex items-center gap-4">
                                    <div className="p-2 rounded-lg bg-primary/10 text-primary">
                                        <Server className="w-5 h-5" />
                                    </div>
                                    <div>
                                        <div className="flex items-center gap-2">
                                            <span className="font-bold">{node.metadata.gpuModel}</span>
                                            {node.nodeId === primaryNode?.nodeId && <Badge variant="secondary" className="text-[10px]">Primary</Badge>}
                                        </div>
                                        <div className="flex items-center gap-3 text-sm text-muted-foreground mt-0.5">
                                            <span className="font-mono text-xs opacity-50">{node.nodeId}</span>
                                            <span>•</span>
                                            <span>{node.metadata.region}</span>
                                            <span>•</span>
                                            <span className={cn(node.status === 'Active' ? 'text-emerald-400' : node.status === 'Pending' ? 'text-yellow-400' : 'text-muted-foreground')}>
                                                {node.status}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <div className="text-right hidden sm:block">
                                    <div className="text-sm font-medium text-muted-foreground">Stake</div>
                                    <div className="font-mono font-bold">{node.stakedAmountEth.toFixed(4)} ETH</div>
                                </div>
                            </CardContent>
                        </Card>
                    ))}
                </div>
            </div>

            <div className="space-y-4">
                <h3 className="text-xl font-bold tracking-tight">Funded Tasks Matching Your Primary Node</h3>
                <div className="grid gap-4">
                    {availableTasks.length === 0 ? (
                        <div className="text-center py-10 text-muted-foreground bg-white/[0.02] rounded-xl border border-white/5">
                            No funded tasks currently match your primary node&apos;s trust and power profile.
                        </div>
                    ) : availableTasks.map((task) => (
                        <Card key={task.taskId} className="bg-white/[0.02] border-white/[0.05] hover:bg-white/[0.04] transition-colors">
                            <CardContent className="p-6 flex items-center justify-between gap-6">
                                <div>
                                    <div className="flex items-center gap-2 mb-1">
                                        <span className="font-bold text-lg">{task.specTitle}</span>
                                        <Badge variant="outline" className="text-[10px]">{task.minTrustLevel}+ Trust</Badge>
                                    </div>
                                    <div className="flex gap-4 text-sm text-muted-foreground">
                                        <span>Power: {task.requiredPower}</span>
                                        <span>Est. Time: {task.durationSeconds / 3600}h</span>
                                    </div>
                                </div>
                                <div className="text-right">
                                    <div className="font-mono font-bold text-emerald-400 text-lg">{task.escrowAmountEth.toFixed(4)} ETH</div>
                                    <Button size="sm" className="mt-2" onClick={() => void handleAcceptTask(task.taskId)} disabled={!primaryNode || !selectedWallet}>
                                        Accept Task
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>
                    ))}
                </div>
            </div>
        </div>
    );
};

export default ProviderView;
