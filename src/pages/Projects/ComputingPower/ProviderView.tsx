
import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { useComputingStore, ResourceType } from './store/useComputingStore';
import { Loader2, Zap, CheckCircle2, Activity, Wallet, Server, MoreHorizontal } from 'lucide-react';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';

const ProviderView: React.FC = () => {
    const { user, nodes, registerNode, submitPoW, acceptTask, submitResult, tasks } = useComputingStore();

    // Registration State
    const [regType, setRegType] = useState<ResourceType>('GPU');
    const [regModel, setRegModel] = useState('');
    const [regRegion, setRegRegion] = useState('US-East');
    const [stakeAmount, setStakeAmount] = useState(0.5);
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [isRegistering, setIsRegistering] = useState(false);

    // PoW State
    const [isMining, setIsMining] = useState(false);

    const myNode = user.providerNodeId ? nodes[user.providerNodeId] : undefined;
    const myNodes = Object.values(nodes).filter(n => n.owner === user.address);
    const availableTasks = Object.values(tasks).filter(t => t.status === 'Open' && (!t.minTrustLevel || (myNode?.trustLevel || 0) >= t.minTrustLevel) && t.resourceType === myNode?.resourceType);
    const myActiveTasks = Object.values(tasks).filter(t => t.assignedNode === myNode?.nodeId && t.status === 'InProgress');

    const handleRegister = async () => {
        setIsRegistering(true);
        try {
            await registerNode(regType, { gpuModel: regModel, region: regRegion }, stakeAmount);
            setIsRegistering(false); // Go back to list after success
        } finally {
            setIsSubmitting(false);
        }
    };

    const handlePoW = async () => {
        if (!myNode) return;
        setIsMining(true);
        try {
            await submitPoW(myNode.nodeId);
        } finally {
            setIsMining(false);
        }
    };

    if (isRegistering) {
        return (
            <div className="max-w-3xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
                <div className="flex items-center gap-4">
                    <Button variant="ghost" size="sm" onClick={() => setIsRegistering(false)}>
                        ← Back to Dashboard
                    </Button>
                </div>
                <div className="text-center space-y-4">
                    <h2 className="text-3xl font-black tracking-tight">Register Your Node</h2>
                    <p className="text-muted-foreground">Join the decentralized network and earn ETH by providing compute power.</p>
                </div>

                <Card className="bg-white/[0.02] border-white/[0.05] py-6">
                    <CardHeader>
                        <CardTitle>Node Configuration</CardTitle>
                        <CardDescription>Define your hardware capabilities and staking amount.</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-6">
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label>Resource Type</Label>
                                <Select value={regType} onValueChange={(v: ResourceType) => setRegType(v)}>
                                    <SelectTrigger>
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="GPU">GPU (Graphics)</SelectItem>
                                        <SelectItem value="CPU">CPU (Processing)</SelectItem>
                                        <SelectItem value="Network">Network (Bandwidth)</SelectItem>
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
                                        <SelectItem value="US-East">US East (N. Virginia)</SelectItem>
                                        <SelectItem value="EU-Central">EU Central (Frankfurt)</SelectItem>
                                        <SelectItem value="Asia-East">Asia East (Tokyo)</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                        </div>

                        <div className="space-y-2">
                            <Label>Hardware Model</Label>
                            <Input placeholder="e.g. RTX 4090, A100 80GB" value={regModel} onChange={e => setRegModel(e.target.value)} />
                        </div>

                        <div className="space-y-4 pt-4 border-t border-white/5">
                            <div className="flex justify-between">
                                <Label>ETH Stake Amount</Label>
                                <span className="font-mono font-bold text-primary">{stakeAmount} ETH</span>
                            </div>
                            <Slider
                                value={[stakeAmount]}
                                onValueChange={v => setStakeAmount(v[0])}
                                min={0.5}
                                max={10}
                                step={0.1}
                                className="[&_.bg-primary]:bg-primary"
                            />
                            <div className="text-xs text-muted-foreground flex justify-between">
                                <span>Min: 0.5 ETH</span>
                                <span className={cn(stakeAmount >= 5 ? "text-emerald-400" : stakeAmount >= 3 ? "text-blue-400" : "text-yellow-400")}>
                                    Trust Level: {stakeAmount >= 5 ? 'Partner' : stakeAmount >= 3 ? 'Trusted' : stakeAmount >= 1 ? 'Verified' : 'Basic'}
                                </span>
                            </div>
                        </div>

                        <Button className="w-full h-12 font-bold text-base mt-2" onClick={handleRegister} disabled={isSubmitting}>
                            {isSubmitting ? <Loader2 className="w-5 h-5 animate-spin mr-2" /> : <Wallet className="w-5 h-5 mr-2" />}
                            Stake {stakeAmount} ETH & Register
                        </Button>
                    </CardContent>
                </Card>
            </div>
        );
    }

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {/* Status Header */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Card className="bg-white/[0.02] border-white/[0.05] py-5">
                    <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Node Status</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="flex items-center justify-between">
                            <div className={cn("text-2xl font-black",
                                myNode ? (myNode.status === 'Active' ? 'text-emerald-400' : 'text-yellow-400') : 'text-muted-foreground'
                            )}>
                                {myNode ? myNode.status : 'N/A'}
                            </div>
                            {myNode && myNode.status === 'Pending' && (
                                <Button size="sm" onClick={handlePoW} disabled={isMining}>
                                    {isMining ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : <Zap className="w-4 h-4 mr-2" />}
                                    Run PoW
                                </Button>
                            )}
                        </div>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05] py-5">
                    <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Trust Level</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-blue-400">{myNode ? `Level ${myNode.trustLevel}` : '-'}</div>
                        <p className="text-xs text-muted-foreground mt-1">Reputation: {myNode ? myNode.reputation : '-'}</p>
                    </CardContent>
                </Card>
                <Card className="bg-white/[0.02] border-white/[0.05] py-5">
                    <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Total Earnings</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-black text-primary">{myNode ? myNode.totalEarnings.toFixed(4) : '0.000'} ETH</div>
                        <p className="text-xs text-muted-foreground mt-1">{myNode ? myNode.totalTasksCompleted : 0} Tasks Completed</p>
                    </CardContent>
                </Card>
            </div>

            {/* Active Task */}
            {myActiveTasks.length > 0 && (
                <Card className="border-primary/50 bg-primary/5 py-4">
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2">
                            <Activity className="w-5 h-5 text-primary animate-pulse" />
                            Running Task: {myActiveTasks[0].specTitle}
                        </CardTitle>
                    </CardHeader>
                    <CardContent className="flex justify-between items-center">
                        <div className="space-y-1">
                            <p className="text-sm text-muted-foreground">Expected Duration: {myActiveTasks[0].duration / 3600}h</p>
                            <p className="text-sm text-muted-foreground">Payout: {(myActiveTasks[0].escrowAmount * 0.92).toFixed(4)} ETH</p>
                        </div>
                        <Button onClick={() => submitResult(myActiveTasks[0].taskId)}>
                            <CheckCircle2 className="w-4 h-4 mr-2" />
                            Submit Result
                        </Button>
                    </CardContent>
                </Card>
            )}

            {/* My Nodes List */}
            <div className="space-y-4">
                <div className="flex items-center justify-between">
                    <h3 className="text-xl font-bold tracking-tight">My Nodes</h3>
                    <Button variant="outline" size="sm" onClick={() => setIsRegistering(true)}>
                        <Server className="w-4 h-4 mr-2" />
                        Register New Node
                    </Button>
                </div>
                <div className="grid gap-4">
                    {myNodes.map(node => (
                        <Card key={node.nodeId} className="bg-white/[0.02] border-white/[0.05] py-4">
                            <CardContent className="flex items-center justify-between">
                                <div className="flex items-center gap-4">
                                    <div className={cn("p-2 rounded-lg bg-primary/10 text-primary")}>
                                        <Server className="w-5 h-5" />
                                    </div>
                                    <div>
                                        <div className="flex items-center gap-2">
                                            <span className="font-bold">{node.metadata.gpuModel}</span>
                                            {node.nodeId === user.providerNodeId && <Badge variant="secondary" className="text-[10px]">Primary</Badge>}
                                        </div>
                                        <div className="flex items-center gap-3 text-sm text-muted-foreground mt-0.5">
                                            <span className="font-mono text-xs opacity-50">{node.nodeId}</span>
                                            <span>•</span>
                                            <span>{node.metadata.region}</span>
                                            <span>•</span>
                                            <span className={cn(
                                                node.status === 'Active' ? "text-emerald-400" :
                                                    node.status === 'Pending' ? "text-yellow-400" : "text-muted-foreground"
                                            )}>{node.status}</span>
                                        </div>
                                    </div>
                                </div>

                                <div className="flex items-center gap-6">
                                    <div className="text-right hidden sm:block">
                                        <div className="text-sm font-medium text-muted-foreground">Earnings</div>
                                        <div className="font-mono font-bold">{node.totalEarnings.toFixed(4)} ETH</div>
                                    </div>
                                    <DropdownMenu>
                                        <DropdownMenuTrigger asChild>
                                            <Button variant="ghost" size="icon" className="h-8 w-8">
                                                <MoreHorizontal className="w-4 h-4" />
                                            </Button>
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent align="end">
                                            <DropdownMenuItem>View Details</DropdownMenuItem>
                                            <DropdownMenuItem>Configuration</DropdownMenuItem>
                                            <DropdownMenuItem className="text-destructive">Stop Node</DropdownMenuItem>
                                        </DropdownMenuContent>
                                    </DropdownMenu>
                                </div>
                            </CardContent>
                        </Card>
                    ))}
                </div>
            </div>

            {/* Available Tasks */}
            <div className="space-y-4">
                <h3 className="text-xl font-bold tracking-tight">Available Tasks Queue</h3>
                <div className="grid gap-4">
                    {availableTasks.length === 0 ? (
                        <div className="text-center py-10 text-muted-foreground bg-white/[0.02] rounded-xl border border-white/5">
                            No tasks available matching your trust level and hardware.
                        </div>
                    ) : (
                        availableTasks.map(task => (
                            <Card key={task.taskId} className="bg-white/[0.02] border-white/[0.05] hover:bg-white/[0.04] transition-colors">
                                <CardContent className="p-6 flex items-center justify-between">
                                    <div>
                                        <div className="flex items-center gap-2 mb-1">
                                            <span className="font-bold text-lg">{task.specTitle}</span>
                                            <Badge variant="outline" className="text-[10px]">{task.minTrustLevel}+ Trust</Badge>
                                        </div>
                                        <div className="flex gap-4 text-sm text-muted-foreground">
                                            <span>Power: {task.requiredPower} TFLOPS</span>
                                            <span>Est. Time: {task.duration / 3600}h</span>
                                        </div>
                                    </div>
                                    <div className="text-right">
                                        <div className="font-mono font-bold text-emerald-400 text-lg">{(task.maxPrice * (task.duration / 3600)).toFixed(4)} ETH</div>
                                        <Button size="sm" className="mt-2" onClick={() => myNode && acceptTask(task.taskId, myNode.nodeId)} disabled={!myNode}>
                                            Accept Task
                                        </Button>
                                    </div>
                                </CardContent>
                            </Card>
                        ))
                    )}
                </div>
            </div>
        </div>
    );
};

export default ProviderView;
