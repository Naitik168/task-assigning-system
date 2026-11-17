import { apiRequest } from './client';
import type { DevEmail, LoginResponse, SeedUsersResponse, TokenResponse, User } from './types';

export const authApi = {
  /** Step 1: email + password -> 2FA challenge (no JWT). */
  login: (email: string, password: string) =>
    apiRequest<LoginResponse>('/auth/login', {
      method: 'POST',
      body: { email, password },
      auth: false,
    }),

  /** Step 2: challenge id + emailed code -> JWT. */
  verifyTwoFactor: (loginChallengeId: string, code: string) =>
    apiRequest<TokenResponse>('/auth/verify-2fa', {
      method: 'POST',
      body: { login_challenge_id: loginChallengeId, code },
      auth: false,
    }),

  me: () => apiRequest<User>('/auth/me'),
};

/** Development-only helpers (these endpoints don't exist in production). */
export const devApi = {
  seedUsers: () =>
    apiRequest<SeedUsersResponse>('/seed/users', { method: 'POST', auth: false }),

  latestEmail: (email: string) =>
    apiRequest<DevEmail>(`/dev/email-logs/latest?email=${encodeURIComponent(email)}`, {
      auth: false,
    }),
};
