import type { ApiErrorBody } from './types';

/** Base URL of the Rust API. Defaults to "/api" (proxied by Vite / nginx). */
export const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL ?? '/api').replace(/\/$/, '');

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }

  get isForbidden() {
    return this.status === 403;
  }
}

interface ClientConfig {
  getToken: () => string | null;
  onUnauthorized: () => void;
}

let clientConfig: ClientConfig = { getToken: () => null, onUnauthorized: () => {} };

/** Called once by AuthProvider so every request can attach the current JWT. */
export function configureApiClient(config: ClientConfig) {
  clientConfig = config;
}

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH';
  body?: unknown;
  /** Attach the Bearer token (default true). */
  auth?: boolean;
}

export async function apiRequest<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, auth = true } = options;
  const headers: Record<string, string> = { Accept: 'application/json' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';

  const token = auth ? clientConfig.getToken() : null;
  if (token) headers.Authorization = `Bearer ${token}`;

  let response: Response;
  try {
    response = await fetch(`${API_BASE_URL}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, 'NETWORK_ERROR', 'Cannot reach the API. Is the backend running?');
  }

  const text = await response.text();
  const data: unknown = text ? safeJson(text) : null;

  if (!response.ok) {
    const err = (data as ApiErrorBody | null)?.error;
    // An expired/invalid session token: drop the session.
    if (response.status === 401 && token) clientConfig.onUnauthorized();
    throw new ApiError(
      response.status,
      err?.code ?? `HTTP_${response.status}`,
      err?.message ?? (response.statusText || 'Request failed'),
    );
  }
  return data as T;
}

function safeJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return error.message;
  if (error instanceof Error) return error.message;
  return 'Something went wrong.';
}
