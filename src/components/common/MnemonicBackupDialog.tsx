import React, { useEffect, useMemo, useState } from 'react';
import { AlertCircle, CheckCircle2, Copy } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  LOCAL_PASSWORD_DEVICE_MESSAGE,
  LOCAL_PASSWORD_RECOVERY_MESSAGE,
  LOCAL_PASSWORD_SYNC_MESSAGE,
} from '@/lib/security';

function buildWordCheckIndexes(words: string[]) {
  if (words.length <= 3) {
    return words.map((_, index) => index);
  }

  const chosen = new Set<number>();

  while (chosen.size < 3) {
    chosen.add(Math.floor(Math.random() * words.length));
  }

  return Array.from(chosen).sort((left, right) => left - right);
}

interface MnemonicBackupDialogProps {
  open: boolean;
  chainLabel: string;
  walletLabel: string;
  mnemonic: string | null;
  copied: boolean;
  isSaving: boolean;
  onCopy: (mnemonic: string) => void;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}

const MnemonicBackupDialog: React.FC<MnemonicBackupDialogProps> = ({
  open,
  chainLabel,
  walletLabel,
  mnemonic,
  copied,
  isSaving,
  onCopy,
  onCancel,
  onConfirm,
}) => {
  const words = useMemo(() => (mnemonic ?? '').split(' ').filter(Boolean), [mnemonic]);
  const [wordChecks, setWordChecks] = useState<number[]>([]);
  const [answers, setAnswers] = useState<Record<number, string>>({});

  useEffect(() => {
    if (!open || words.length === 0) {
      setWordChecks([]);
      setAnswers({});
      return;
    }

    const nextChecks = buildWordCheckIndexes(words);
    setWordChecks(nextChecks);
    setAnswers(
      nextChecks.reduce<Record<number, string>>((result, index) => {
        result[index] = '';
        return result;
      }, {})
    );
  }, [open, words]);

  const checksPass = wordChecks.every((index) => {
    return answers[index]?.trim().toLowerCase() === words[index]?.toLowerCase();
  });

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onCancel()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-[640px]">
        <DialogHeader className="space-y-3">
          <DialogTitle>Back Up Recovery Phrase Before Wallet Save</DialogTitle>
          <DialogDescription>
            Write down this {chainLabel} recovery phrase before the wallet is encrypted and saved locally.
          </DialogDescription>
        </DialogHeader>

        {mnemonic && (
          <div className="space-y-6">
            <div className="rounded-lg border border-red-200 bg-red-50 p-4">
              <div className="flex items-start gap-3">
                <AlertCircle className="mt-0.5 h-5 w-5 flex-shrink-0 text-red-600" />
                <div className="space-y-2 text-sm text-red-900">
                  <p className="font-semibold">Finish this backup before entering the wallet.</p>
                  <p>{LOCAL_PASSWORD_DEVICE_MESSAGE}</p>
                  <p>{LOCAL_PASSWORD_RECOVERY_MESSAGE}</p>
                  <p>{LOCAL_PASSWORD_SYNC_MESSAGE}</p>
                </div>
              </div>
            </div>

            <div className="rounded-lg border border-blue-200 bg-blue-50 p-4">
              <p className="text-sm text-muted-foreground">Wallet Label</p>
              <p className="mt-1 font-medium text-slate-950">{walletLabel || `${chainLabel} Wallet`}</p>
            </div>

            <div className="space-y-2">
              <Label>Recovery Phrase</Label>
              <div className="space-y-3 rounded-lg bg-muted p-4">
                <div className="grid grid-cols-3 gap-2">
                  {words.map((word, index) => (
                    <div key={`${word}-${index}`} className="flex items-center gap-2">
                      <span className="w-6 text-xs font-mono text-muted-foreground">{index + 1}.</span>
                      <span className="font-mono text-sm text-yellow-400">{word}</span>
                    </div>
                  ))}
                </div>
                <Button type="button" variant="outline" className="w-full" onClick={() => onCopy(mnemonic)}>
                  {copied ? <CheckCircle2 className="mr-2 h-4 w-4" /> : <Copy className="mr-2 h-4 w-4" />}
                  {copied ? 'Copied' : 'Copy Recovery Phrase'}
                </Button>
              </div>
            </div>

            <div className="space-y-3 rounded-lg border border-yellow-200 bg-yellow-50 p-4">
              <p className="text-sm font-semibold text-yellow-900">Random word check</p>
              <p className="text-sm text-yellow-800">
                Enter the requested words to prove the phrase is backed up before local encryption and save continue.
              </p>
              <div className="grid gap-3 sm:grid-cols-3">
                {wordChecks.map((index) => (
                  <div key={`check-${index}`} className="space-y-2">
                    <Label htmlFor={`mnemonic-check-${index}`}>Word {index + 1}</Label>
                    <Input
                      id={`mnemonic-check-${index}`}
                      value={answers[index] ?? ''}
                      onChange={(event) => {
                        const nextValue = event.target.value;
                        setAnswers((current) => ({
                          ...current,
                          [index]: nextValue,
                        }));
                      }}
                      placeholder={`Enter word ${index + 1}`}
                    />
                  </div>
                ))}
              </div>
            </div>

            <div className="rounded-lg border border-yellow-200 bg-yellow-50 p-4 text-sm text-yellow-900">
              Forgetting the local password does not recover funds. The only supported fallback is to reset local wallet data on this device and restore with this recovery phrase or the original private key.
            </div>

            <div className="flex gap-3">
              <Button type="button" variant="outline" className="flex-1" onClick={onCancel} disabled={isSaving}>
                Cancel
              </Button>
              <Button
                type="button"
                className="flex-1 bg-green-600 hover:bg-green-700"
                onClick={() => void onConfirm()}
                disabled={!checksPass || isSaving}
              >
                {isSaving ? 'Encrypting and Saving...' : 'Encrypt And Save Wallet'}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
};

export { MnemonicBackupDialog };
