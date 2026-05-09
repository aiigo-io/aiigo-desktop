import React, { useState } from 'react';

import { useSecuritySession } from '@/components/common/SecuritySession';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { AlertTriangle, CheckCircle, Clock, DollarSign, FileText, Plus } from 'lucide-react';
import { toast } from 'sonner';

import { cn } from '@/lib/utils';
import type { ComputeMarketplaceModel, ResourceType } from './useComputeMarketplace';

const BuyerView: React.FC<{ model: ComputeMarketplaceModel }> = ({ model }) => {
    const { requestUnlock } = useSecuritySession();
    const { selectedWallet, config, snapshot, createAndFundTask, approveTask, disputeTask } = model;

    const [specTitle, setSpecTitle] = useState('');
    const [resourceType, setResourceType] = useState<ResourceType>('GPU');
    const [durationHours, setDurationHours] = useState(1);
    const [requiredPower, setRequiredPower] = useState(80);
    const [maxPrice, setMaxPrice] = useState('0.005');
    const [disputeReasons, setDisputeReasons] = useState<Record<string, string>>({});

    const myTasks = snapshot.buyerTasks;

    const authorizeSend = async (prompt: string) => requestUnlock({ operation: 'send', prompt });

    const handleCreate = async () => {
        try {
            if (!(await authorizeSend('Authorize task creation and escrow funding'))) {
                return;
            }

            await createAndFundTask({
                resourceType,
                requiredPower,
                durationHours,
                maxPriceEthPerHour: maxPrice,
                minTrustLevel: 2,
                specTitle,
            });
            setSpecTitle('');
            toast.success('Task creation and escrow funding confirmed.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    const handleApprove = async (taskId: string) => {
        try {
            if (!(await authorizeSend('Authorize buyer approval and escrow release'))) {
                return;
            }

            await approveTask(taskId);
            toast.success('Approval transaction confirmed.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    const handleDispute = async (taskId: string) => {
        try {
            const reason = disputeReasons[taskId]?.trim() || 'Buyer requested dispute review';
            if (!(await authorizeSend('Authorize dispute transaction'))) {
                return;
            }

            await disputeTask(taskId, reason);
            toast.success('Dispute transaction confirmed.');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error));
        }
    };

    return (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="lg:col-span-1 space-y-6">
                <Card className="bg-white/[0.02] border-white/[0.05] sticky top-6 py-6">
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2">
                            <Plus className="w-5 h-5 text-primary" /> Create Funded Task
                        </CardTitle>
                        <CardDescription>This sends two real transactions: create task, then fund escrow. No local-only completion path remains.</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="space-y-2">
                            <Label>Task Title</Label>
                            <Input placeholder="e.g. Llama-3 Training" value={specTitle} onChange={(event) => setSpecTitle(event.target.value)} />
                        </div>

                        <div className="grid grid-cols-2 gap-3">
                            <div className="space-y-2">
                                <Label>Type</Label>
                                <Select value={resourceType} onValueChange={(value: ResourceType) => setResourceType(value)}>
                                    <SelectTrigger><SelectValue /></SelectTrigger>
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
                                <Label>Duration (Hrs)</Label>
                                <Input type="number" min={1} value={durationHours} onChange={(event) => setDurationHours(Number(event.target.value))} />
                            </div>
                        </div>

                        <div className="grid grid-cols-2 gap-3">
                            <div className="space-y-2">
                                <Label>Required Power</Label>
                                <Input type="number" min={1} value={requiredPower} onChange={(event) => setRequiredPower(Number(event.target.value))} />
                            </div>
                            <div className="space-y-2">
                                <Label>Max Price (ETH/hr)</Label>
                                <Input type="number" min="0" step="0.0001" value={maxPrice} onChange={(event) => setMaxPrice(event.target.value)} />
                            </div>
                        </div>

                        <div className="pt-4 border-t border-white/5 space-y-4">
                            <div className="flex justify-between text-sm">
                                <span className="text-muted-foreground">Total Escrow Required:</span>
                                <span className="font-mono font-bold text-primary">{(Number(maxPrice || '0') * durationHours).toFixed(4)} ETH</span>
                            </div>
                            <div className="flex justify-between text-sm">
                                <span className="text-muted-foreground">Target chain:</span>
                                <span className="font-medium">{config.chainName}</span>
                            </div>
                            <Button className="w-full font-bold" onClick={() => void handleCreate()} disabled={!config.isConfigured || !selectedWallet || !specTitle.trim()}>
                                <DollarSign className="w-4 h-4 mr-2" />
                                Create & Fund Escrow
                            </Button>
                        </div>
                    </CardContent>
                </Card>
            </div>

            <div className="lg:col-span-2 space-y-6">
                <h3 className="text-xl font-bold tracking-tight">Your Tasks</h3>
                <div className="space-y-4">
                    {myTasks.length === 0 ? (
                        <div className="text-center py-12 text-muted-foreground bg-white/[0.02] rounded-xl border border-white/5">
                            No tasks found for the selected wallet in the current snapshot.
                        </div>
                    ) : myTasks.map((task) => (
                        <Card key={task.taskId} className="bg-white/[0.02] border-white/[0.05]">
                            <CardContent className="p-6">
                                <div className="flex justify-between items-start mb-4 gap-6">
                                    <div>
                                        <h4 className="font-bold text-lg mb-1">{task.specTitle}</h4>
                                        <div className="flex gap-2">
                                            <Badge variant="secondary" className="text-[10px]">{task.resourceType}</Badge>
                                            <Badge variant="outline" className={cn('text-[10px]', task.status === 'Completed' ? 'border-emerald-500 text-emerald-500' : task.status === 'Assigned' || task.status === 'InProgress' ? 'border-blue-500 text-blue-500' : '')}>
                                                {task.status}
                                            </Badge>
                                        </div>
                                    </div>
                                    <div className="text-right text-sm text-muted-foreground">
                                        <div className="flex items-center gap-1 justify-end"><Clock className="w-3 h-3" /> {(task.durationSeconds / 3600).toFixed(1)}h</div>
                                        <div className="font-mono mt-1">{task.escrowAmountEth.toFixed(4)} ETH</div>
                                    </div>
                                </div>

                                {(task.status === 'Assigned' || task.status === 'InProgress') && (
                                    <div className="text-sm bg-blue-500/10 text-blue-400 p-3 rounded-md border border-blue-500/20 flex items-center gap-2">
                                        <FileText className="w-4 h-4" />
                                        Provider is executing the funded task on-chain.
                                    </div>
                                )}

                                {task.status === 'Completed' && (
                                    <div className="mt-4 p-4 bg-black/20 rounded-lg border border-white/10 space-y-3">
                                        <div className="flex items-center gap-2 text-emerald-400 font-bold mb-2">
                                            <FileText className="w-4 h-4" />
                                            Result Submitted
                                        </div>
                                        <p className="text-xs text-muted-foreground">Challenge deadline: {task.challengeDeadline ? new Date(task.challengeDeadline).toLocaleString() : 'Pending'}</p>
                                        <div className="space-y-2">
                                            <Input
                                                placeholder="Optional dispute reason"
                                                value={disputeReasons[task.taskId] ?? ''}
                                                onChange={(event) => setDisputeReasons((current) => ({ ...current, [task.taskId]: event.target.value }))}
                                            />
                                            <div className="flex gap-2">
                                                <Button size="sm" className="w-full bg-emerald-600 hover:bg-emerald-700" onClick={() => void handleApprove(task.taskId)}>
                                                    <CheckCircle className="w-4 h-4 mr-2" /> Approve & Release Payment
                                                </Button>
                                                <Button size="sm" variant="destructive" className="w-full" onClick={() => void handleDispute(task.taskId)}>
                                                    <AlertTriangle className="w-4 h-4 mr-2" /> Dispute
                                                </Button>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {task.status === 'Verified' && (
                                    <div className="text-sm bg-emerald-500/10 text-emerald-400 p-3 rounded-md border border-emerald-500/20 flex items-center gap-2 mt-4">
                                        <CheckCircle className="w-4 h-4" />
                                        Task settled on-chain.
                                    </div>
                                )}

                                {task.status === 'Disputed' && (
                                    <div className="text-sm bg-amber-500/10 text-amber-300 p-3 rounded-md border border-amber-500/20 flex items-center gap-2 mt-4">
                                        <AlertTriangle className="w-4 h-4" />
                                        Dispute opened{task.disputeReason ? `: ${task.disputeReason}` : '.'}
                                    </div>
                                )}
                            </CardContent>
                        </Card>
                    ))}
                </div>
            </div>
        </div>
    );
};

export default BuyerView;
