
import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useComputingStore, ResourceType } from './store/useComputingStore';
import { Loader2, Plus, CheckCircle, AlertTriangle, FileText, Clock, DollarSign } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';

const BuyerView: React.FC = () => {
    const { user, tasks, createTask, verifyResult, disputeTask } = useComputingStore();

    // Create Task State
    const [specTitle, setSpecTitle] = useState('');
    const [resType, setResType] = useState<ResourceType>('GPU');
    const [duration, setDuration] = useState(1); // hours
    const [power, setPower] = useState(80); // TFLOPS
    const [price, setPrice] = useState(0.005); // ETH/hr
    const [isCreating, setIsCreating] = useState(false);

    const myTasks = Object.values(tasks).filter(t => t.buyer === user.address).sort((a, b) => b.createdAt - a.createdAt);

    const handleCreate = async () => {
        setIsCreating(true);
        try {
            await createTask({
                resourceType: resType,
                requiredPower: power,
                duration: duration * 3600,
                maxPrice: price,
                minTrustLevel: 2,
                specTitle,
            });
            setSpecTitle('');
        } finally {
            setIsCreating(false);
        }
    };

    const VerificationView = ({ task }: { task: any }) => (
        <div className="mt-4 p-4 bg-black/20 rounded-lg border border-white/10 space-y-3">
            <div className="flex items-center gap-2 text-emerald-400 font-bold mb-2">
                <FileText className="w-4 h-4" />
                Result Submitted
            </div>
            <p className="text-xs font-mono bg-black/40 p-2 rounded text-muted-foreground truncate">
                Result Hash: 0x8a9 ... 3b2d
            </p>
            <div className="flex gap-2">
                <Button size="sm" className="w-full bg-emerald-600 hover:bg-emerald-700" onClick={() => verifyResult(task.taskId)}>
                    <CheckCircle className="w-4 h-4 mr-2" /> Approve & Release Payment
                </Button>
                <Button size="sm" variant="destructive" className="w-full" onClick={() => disputeTask(task.taskId)}>
                    <AlertTriangle className="w-4 h-4 mr-2" /> Dispute
                </Button>
            </div>
        </div>
    );

    return (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {/* Create Task Form */}
            <div className="lg:col-span-1 space-y-6">
                <Card className="bg-white/[0.02] border-white/[0.05] sticky top-6 py-6">
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2">
                            <Plus className="w-5 h-5 text-primary" /> Create New Task
                        </CardTitle>
                        <CardDescription>Define requirements and deposit escrow.</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="space-y-2">
                            <Label>Task Title</Label>
                            <Input placeholder="e.g. Llama-3 Training" value={specTitle} onChange={e => setSpecTitle(e.target.value)} />
                        </div>

                        <div className="grid grid-cols-2 gap-3">
                            <div className="space-y-2">
                                <Label>Type</Label>
                                <Select value={resType} onValueChange={(v: ResourceType) => setResType(v)}>
                                    <SelectTrigger><SelectValue /></SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="GPU">GPU</SelectItem>
                                        <SelectItem value="CPU">CPU</SelectItem>
                                        <SelectItem value="Network">Network</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                            <div className="space-y-2">
                                <Label>Duration (Hrs)</Label>
                                <Input type="number" value={duration} onChange={e => setDuration(Number(e.target.value))} />
                            </div>
                        </div>

                        <div className="grid grid-cols-2 gap-3">
                            <div className="space-y-2">
                                <Label>Min Power (TFLOPS)</Label>
                                <Input type="number" value={power} onChange={e => setPower(Number(e.target.value))} />
                            </div>
                            <div className="space-y-2">
                                <Label>Max Price (ETH/hr)</Label>
                                <Input type="number" value={price} onChange={e => setPrice(Number(e.target.value))} />
                            </div>
                        </div>

                        <div className="pt-4 border-t border-white/5 space-y-4">
                            <div className="flex justify-between text-sm">
                                <span className="text-muted-foreground">Total Escrow Required:</span>
                                <span className="font-mono font-bold text-primary">{(price * duration).toFixed(4)} ETH</span>
                            </div>
                            <Button className="w-full font-bold" onClick={handleCreate} disabled={!specTitle || isCreating}>
                                {isCreating ? <Loader2 className="w-4 h-4 animate-spin" /> : <DollarSign className="w-4 h-4 mr-2" />}
                                Deposit & Create Task
                            </Button>
                        </div>
                    </CardContent>
                </Card>
            </div>

            {/* Task List */}
            <div className="lg:col-span-2 space-y-6">
                <h3 className="text-xl font-bold tracking-tight">Your Tasks</h3>
                <div className="space-y-4">
                    {myTasks.length === 0 ? (
                        <div className="text-center py-12 text-muted-foreground bg-white/[0.02] rounded-xl border border-white/5">
                            You haven't created any tasks yet.
                        </div>
                    ) : (
                        myTasks.map(task => (
                            <Card key={task.taskId} className="bg-white/[0.02] border-white/[0.05]">
                                <CardContent className="p-6">
                                    <div className="flex justify-between items-start mb-4">
                                        <div>
                                            <h4 className="font-bold text-lg mb-1">{task.specTitle}</h4>
                                            <div className="flex gap-2">
                                                <Badge variant="secondary" className="text-[10px]">{task.resourceType}</Badge>
                                                <Badge variant="outline" className={cn("text-[10px]",
                                                    task.status === 'Completed' ? 'border-emerald-500 text-emerald-500' :
                                                        task.status === 'InProgress' ? 'border-blue-500 text-blue-500' : ''
                                                )}>
                                                    {task.status}
                                                </Badge>
                                            </div>
                                        </div>
                                        <div className="text-right text-sm text-muted-foreground">
                                            <div className="flex items-center gap-1"><Clock className="w-3 h-3" /> {(task.duration / 3600).toFixed(1)}h</div>
                                            <div className="font-mono mt-1">{(task.escrowAmount).toFixed(4)} ETH</div>
                                        </div>
                                    </div>

                                    {/* Action Areas based on Status */}
                                    {task.status === 'InProgress' && (
                                        <div className="text-sm bg-blue-500/10 text-blue-400 p-3 rounded-md border border-blue-500/20 flex items-center gap-2">
                                            <Loader2 className="w-4 h-4 animate-spin" />
                                            Node executing task... (Started: {new Date(task.startedAt!).toLocaleTimeString()})
                                        </div>
                                    )}

                                    {task.status === 'Completed' && (
                                        <VerificationView task={task} />
                                    )}

                                    {task.status === 'Verified' && (
                                        <div className="text-sm bg-emerald-500/10 text-emerald-400 p-3 rounded-md border border-emerald-500/20 flex items-center gap-2">
                                            <CheckCircle className="w-4 h-4" />
                                            Payment Released. Task closed.
                                        </div>
                                    )}
                                </CardContent>
                            </Card>
                        ))
                    )}
                </div>
            </div>
        </div>
    );
};

export default BuyerView;
