import type { ReactNode } from 'react';

type AlertKind = 'error' | 'success' | 'info' | 'warning';

interface AlertProps {
  kind: AlertKind;
  title?: string;
  children: ReactNode;
  onDismiss?: () => void;
}

export function Alert({ kind, title, children, onDismiss }: AlertProps) {
  return (
    <div
      className={`alert alert-${kind}`}
      role={kind === 'error' || kind === 'warning' ? 'alert' : 'status'}
    >
      <div>
        {title && <strong className="alert-title">{title}</strong>}
        <div>{children}</div>
      </div>
      {onDismiss && (
        <button type="button" className="alert-close" aria-label="Dismiss" onClick={onDismiss}>
          ×
        </button>
      )}
    </div>
  );
}
