import { createContext, useCallback, useContext, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { configureApiClient } from '../api/client';
import type { TokenResponse, User } from '../api/types';

interface Session {
  token: string;
  user: User;
}

interface AuthContextValue {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  signIn: (tokenResponse: TokenResponse) => void;
  signOut: () => void;
}

const STORAGE_KEY = 'taskmgr.session';
const AuthContext = createContext<AuthContextValue | null>(null);

function loadSession(): Session | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Session) : null;
  } catch {
    return null;
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<Session | null>(loadSession);
  // A ref lets the API client read the latest token without re-configuring.
  const sessionRef = useRef(session);
  sessionRef.current = session;

  const signOut = useCallback(() => {
    sessionStorage.removeItem(STORAGE_KEY);
    setSession(null);
  }, []);

  const signIn = useCallback((res: TokenResponse) => {
    const next = { token: res.access_token, user: res.user };
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    setSession(next);
  }, []);

  // Configured during render (not in an effect): child effects run before
  // parent effects, so pages fetching on mount must already see the token.
  useState(() =>
    configureApiClient({
      getToken: () => sessionRef.current?.token ?? null,
      onUnauthorized: () => {
        sessionStorage.removeItem(STORAGE_KEY);
        setSession(null);
      },
    }),
  );

  const value = useMemo<AuthContextValue>(
    () => ({
      user: session?.user ?? null,
      token: session?.token ?? null,
      isAuthenticated: session !== null,
      signIn,
      signOut,
    }),
    [session, signIn, signOut],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used inside <AuthProvider>');
  return ctx;
}

export function homePathFor(user: User | null): string {
  return user?.role === 'admin' ? '/admin' : '/my-tasks';
}
