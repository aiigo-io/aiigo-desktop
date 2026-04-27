import React, { cloneElement, isValidElement } from 'react';
import { useSecuritySession } from '@/components/common/SecuritySession';
import type { SignerOperation } from '@/lib/security';
import { cn } from '@/lib/utils';

interface UnlockGateProps {
  children: React.ReactElement<{
    className?: string;
    onClick?: React.MouseEventHandler<HTMLElement>;
  }>;
  className?: string;
  onUnlockSuccess?: () => void | Promise<void>;
  prompt?: string;
  mode?: 'unlock' | 'setup' | 'reauth';
  operation?: SignerOperation;
}

const UnlockGate: React.FC<UnlockGateProps> = ({
  children,
  className,
  onUnlockSuccess,
  prompt = 'Unlock required',
  mode = 'unlock',
  operation,
}) => {
  const { requestUnlock } = useSecuritySession();

  if (!isValidElement(children)) {
    return null;
  }

  const originalOnClick = children.props.onClick;

  return cloneElement(children, {
    className: className
      ? cn((children.props as { className?: string }).className, className)
      : (children.props as { className?: string }).className,
    onClick: async (event: React.MouseEvent<HTMLElement>) => {
      event.preventDefault();
      event.stopPropagation();

      await requestUnlock({
        prompt,
        mode,
        operation,
        onUnlockSuccess: onUnlockSuccess ?? (() => Promise.resolve(originalOnClick?.(event))),
      });
    },
  });
};

export { UnlockGate };
