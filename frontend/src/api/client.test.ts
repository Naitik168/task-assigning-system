import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, apiRequest, configureApiClient } from './client';
import { jsonResponse } from '../test/utils';

describe('apiRequest', () => {
  afterEach(() => vi.restoreAllMocks());

  it('attaches the JWT as a Bearer token', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(jsonResponse({ ok: true }));
    configureApiClient({ getToken: () => 'jwt-123', onUnauthorized: () => {} });

    await apiRequest('/tasks/view-my-tasks');

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/tasks/view-my-tasks');
    expect((init?.headers as Record<string, string>).Authorization).toBe('Bearer jwt-123');
  });

  it('omits the token for unauthenticated calls', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(jsonResponse({}));
    configureApiClient({ getToken: () => 'jwt-123', onUnauthorized: () => {} });

    await apiRequest('/auth/login', { method: 'POST', body: {}, auth: false });

    expect((fetchMock.mock.calls[0][1]?.headers as Record<string, string>).Authorization).toBeUndefined();
  });

  it('maps API error bodies to ApiError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      jsonResponse({ error: { code: 'FORBIDDEN', message: 'Only admins can perform this action.' } }, 403),
    );
    configureApiClient({ getToken: () => 'jwt', onUnauthorized: () => {} });

    const err = (await apiRequest("/tasks", { method: "POST", body: {} }).catch((e: unknown) => e)) as ApiError;
    expect(err).toBeInstanceOf(ApiError);
    expect(err.status).toBe(403);
    expect(err.code).toBe('FORBIDDEN');
    expect(err.isForbidden).toBe(true);
  });

  it('signs out when an authenticated request returns 401', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      jsonResponse({ error: { code: 'UNAUTHORIZED', message: 'Invalid or expired token.' } }, 401),
    );
    const onUnauthorized = vi.fn();
    configureApiClient({ getToken: () => 'expired', onUnauthorized });

    await expect(apiRequest('/tasks/view-my-tasks')).rejects.toThrow('Invalid or expired token.');
    expect(onUnauthorized).toHaveBeenCalled();
  });

  it('reports network failures clearly', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('fetch failed'));
    await expect(apiRequest('/health', { auth: false })).rejects.toMatchObject({ code: 'NETWORK_ERROR' });
  });
});
