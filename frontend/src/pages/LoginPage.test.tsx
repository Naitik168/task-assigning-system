import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { bondUser, jsonResponse, renderApp } from '../test/utils';

const myTasks = {
  user: { email: bondUser.email, role: 'staff' },
  tasks: [],
  summary: { total_assigned_tasks: 0 },
  cache: { hit: false },
};

describe('LoginPage', () => {
  afterEach(() => vi.restoreAllMocks());

  it('completes password + 2FA login and stores the JWT', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      const url = String(input);
      if (url.endsWith('/auth/login'))
        return jsonResponse({ login_challenge_id: 'ch-1', message: 'Code sent.', expires_in_seconds: 300 });
      if (url.includes('/dev/email-logs/latest'))
        return jsonResponse({ body: 'Your code is 424242.', verification_code: '424242' });
      if (url.endsWith('/auth/verify-2fa'))
        return jsonResponse({ access_token: 'jwt-bond', token_type: 'Bearer', expires_in_seconds: 3600, user: bondUser });
      if (url.endsWith('/tasks/view-my-tasks')) return jsonResponse(myTasks);
      throw new Error(`unexpected ${url}`);
    });
    const user = userEvent.setup();
    renderApp('/login');

    await user.type(screen.getByLabelText('Email'), bondUser.email);
    await user.type(screen.getByLabelText('Password'), 'Bond@007007');
    await user.click(screen.getByRole('button', { name: 'Continue' }));

    // Step 2: code entry; no token yet.
    expect(await screen.findByLabelText('Verification code')).toBeInTheDocument();
    expect(sessionStorage.getItem('taskmgr.session')).toBeNull();

    await user.click(screen.getByRole('button', { name: /fetch code from email log/i }));
    await waitFor(() => expect(screen.getByLabelText('Verification code')).toHaveValue('424242'));
    await user.click(screen.getByRole('button', { name: /verify & sign in/i }));

    expect(await screen.findByText('My assigned tasks')).toBeInTheDocument();
    expect(JSON.parse(sessionStorage.getItem('taskmgr.session')!).token).toBe('jwt-bond');

    const verifyCall = fetchMock.mock.calls.find(([u]) => String(u).endsWith('/auth/verify-2fa'))!;
    expect(JSON.parse(String(verifyCall[1]?.body))).toEqual({ login_challenge_id: 'ch-1', code: '424242' });
  });

  it('shows the API error for an incorrect code', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      const url = String(input);
      if (url.endsWith('/auth/login'))
        return jsonResponse({ login_challenge_id: 'ch-1', message: 'Code sent.', expires_in_seconds: 300 });
      return jsonResponse({ error: { code: 'INVALID_CODE', message: 'The verification code is incorrect.' } }, 401);
    });
    const user = userEvent.setup();
    renderApp('/login');

    await user.type(screen.getByLabelText('Email'), 'admin@example.com');
    await user.type(screen.getByLabelText('Password'), 'x');
    await user.click(screen.getByRole('button', { name: 'Continue' }));
    await user.type(await screen.findByLabelText('Verification code'), '000000');
    await user.click(screen.getByRole('button', { name: /verify & sign in/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The verification code is incorrect.');
  });

  it('shows invalid credentials errors', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      jsonResponse({ error: { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password.' } }, 401),
    );
    const user = userEvent.setup();
    renderApp('/login');

    await user.type(screen.getByLabelText('Email'), 'admin@example.com');
    await user.type(screen.getByLabelText('Password'), 'wrong');
    await user.click(screen.getByRole('button', { name: 'Continue' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid email or password.');
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
  });
});
