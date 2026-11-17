import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { App } from '../App';
import type { User } from '../api/types';
import { AuthProvider } from '../context/AuthContext';

export const adminUser: User = {
  id: 'a1',
  full_name: 'Admin',
  email: 'admin@example.com',
  role: 'admin',
  created_at: '2026-01-01T00:00:00Z',
};

export const bondUser: User = {
  id: 'b1',
  full_name: 'James Bond',
  email: 'jamesbond@example.com',
  role: 'staff',
  created_at: '2026-01-01T00:00:00Z',
};

/** Renders the full app at `path`, optionally with an existing session. */
export function renderApp(path: string, session?: { user: User; token?: string }) {
  if (session) {
    sessionStorage.setItem(
      'taskmgr.session',
      JSON.stringify({ token: session.token ?? 'test-token', user: session.user }),
    );
  }
  return render(
    <AuthProvider>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </AuthProvider>,
  );
}

export function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}
