import type { ReactNode } from 'react';
import { Navigate } from 'react-router-dom';
import type { Role } from '../api/types';
import { homePathFor, useAuth } from '../context/AuthContext';

/** Redirects to /login when signed out, or to the user's home when the role doesn't match. */
export function ProtectedRoute({ role, children }: { role?: Role; children: ReactNode }) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" replace />;
  if (role && user.role !== role) return <Navigate to={homePathFor(user)} replace />;
  return <>{children}</>;
}
