import { useState } from 'react';
import { Navigate, useNavigate } from 'react-router-dom';
import { authApi, devApi } from '../api/auth';
import { errorMessage } from '../api/client';
import { Alert } from '../components/Alert';
import { LoginForm } from '../components/LoginForm';
import { TwoFactorForm } from '../components/TwoFactorForm';
import { homePathFor, useAuth } from '../context/AuthContext';

type Step =
  | { kind: 'credentials' }
  | { kind: 'code'; email: string; challengeId: string; expiresIn: number };

export function LoginPage() {
  const { user, signIn } = useAuth();
  const navigate = useNavigate();
  const [step, setStep] = useState<Step>({ kind: 'credentials' });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  if (user) return <Navigate to={homePathFor(user)} replace />;

  async function handleCredentials(email: string, password: string) {
    setSubmitting(true);
    setError(null);
    setNotice(null);
    try {
      const res = await authApi.login(email, password);
      setStep({ kind: 'code', email, challengeId: res.login_challenge_id, expiresIn: res.expires_in_seconds });
      setNotice(res.message);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSubmitting(false);
    }
  }

  async function handleCode(code: string) {
    if (step.kind !== 'code') return;
    setSubmitting(true);
    setError(null);
    try {
      const res = await authApi.verifyTwoFactor(step.challengeId, code);
      signIn(res);
      navigate(homePathFor(res.user), { replace: true });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSubmitting(false);
    }
  }

  async function fetchDevCode(): Promise<string | null> {
    if (step.kind !== 'code') return null;
    try {
      const email = await devApi.latestEmail(step.email);
      setNotice(`Dev email log: “${email.body}”`);
      return email.verification_code;
    } catch (e) {
      setError(errorMessage(e));
      return null;
    }
  }

  async function seedUsers() {
    setError(null);
    try {
      const res = await devApi.seedUsers();
      const summary = res.users
        .map((u) => `${u.email} (${u.role}${u.created ? ', created' : ', already existed'})`)
        .join(', ');
      setNotice(`Seeded users: ${summary}`);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <main className="auth-page">
      <section className="card auth-card">
        <h1>Sign in</h1>
        <p className="muted">
          {step.kind === 'credentials' ? 'Step 1 of 2 · Email and password' : 'Step 2 of 2 · Email verification'}
        </p>

        {error && (
          <Alert kind="error" onDismiss={() => setError(null)}>
            {error}
          </Alert>
        )}
        {notice && !error && <Alert kind="info">{notice}</Alert>}

        {step.kind === 'credentials' ? (
          <LoginForm onSubmit={handleCredentials} submitting={submitting} />
        ) : (
          <TwoFactorForm
            email={step.email}
            expiresInSeconds={step.expiresIn}
            submitting={submitting}
            onSubmit={handleCode}
            onFetchDevCode={fetchDevCode}
            onBack={() => {
              setStep({ kind: 'credentials' });
              setError(null);
              setNotice(null);
            }}
          />
        )}

        {step.kind === 'credentials' && (
          <div className="dev-tools">
            <span className="muted small">First run?</span>
            <button type="button" className="btn btn-link" onClick={seedUsers}>
              Dev: seed Admin & James Bond
            </button>
          </div>
        )}
      </section>
    </main>
  );
}
