import { useState } from 'react';
import type { FormEvent } from 'react';

interface LoginFormProps {
  onSubmit: (email: string, password: string) => void;
  submitting: boolean;
}

const DEMO_ACCOUNTS = [
  { label: 'Admin', email: 'admin@example.com', password: 'Admin@12345' },
  { label: 'James Bond', email: 'jamesbond@example.com', password: 'Bond@007007' },
];

export function LoginForm({ onSubmit, submitting }: LoginFormProps) {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    onSubmit(email.trim(), password);
  }

  return (
    <form className="stack" onSubmit={handleSubmit}>
      <label>
        Email
        <input
          type="email"
          autoComplete="username"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          required
        />
      </label>
      <label>
        Password
        <input
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
        />
      </label>
      <button type="submit" className="btn btn-primary" disabled={submitting}>
        {submitting ? 'Checking credentials…' : 'Continue'}
      </button>
      <div className="demo-accounts">
        <span className="muted small">Demo accounts:</span>
        {DEMO_ACCOUNTS.map((a) => (
          <button
            key={a.email}
            type="button"
            className="btn btn-link"
            onClick={() => {
              setEmail(a.email);
              setPassword(a.password);
            }}
          >
            {a.label}
          </button>
        ))}
      </div>
    </form>
  );
}
