import { useState } from 'react';
import type { FormEvent } from 'react';

interface TwoFactorFormProps {
  email: string;
  expiresInSeconds: number;
  submitting: boolean;
  onSubmit: (code: string) => void;
  onBack: () => void;
  /** Development helper: reads the code from GET /dev/email-logs/latest. */
  onFetchDevCode?: () => Promise<string | null>;
}

export function TwoFactorForm({
  email,
  expiresInSeconds,
  submitting,
  onSubmit,
  onBack,
  onFetchDevCode,
}: TwoFactorFormProps) {
  const [code, setCode] = useState('');
  const [fetching, setFetching] = useState(false);

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    onSubmit(code);
  }

  async function fillFromDevLog() {
    if (!onFetchDevCode) return;
    setFetching(true);
    try {
      const devCode = await onFetchDevCode();
      if (devCode) setCode(devCode);
    } finally {
      setFetching(false);
    }
  }

  return (
    <form className="stack" onSubmit={handleSubmit}>
      <p>
        We emailed a 6-digit code to <strong>{email}</strong>. It expires in{' '}
        {Math.round(expiresInSeconds / 60)} minutes and can be used once.
      </p>
      <label>
        Verification code
        <input
          inputMode="numeric"
          autoComplete="one-time-code"
          pattern="\d{6}"
          maxLength={6}
          value={code}
          onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
          placeholder="123456"
          required
          autoFocus
        />
      </label>
      <button type="submit" className="btn btn-primary" disabled={submitting || code.length !== 6}>
        {submitting ? 'Verifying…' : 'Verify & sign in'}
      </button>
      <div className="row">
        <button type="button" className="btn btn-link" onClick={onBack}>
          ← Use a different account
        </button>
        {onFetchDevCode && (
          <button type="button" className="btn btn-link" onClick={fillFromDevLog} disabled={fetching}>
            {fetching ? 'Reading dev email log…' : 'Dev: fetch code from email log'}
          </button>
        )}
      </div>
    </form>
  );
}
