import { NavLink, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

export function NavBar() {
  const { user, signOut } = useAuth();
  const navigate = useNavigate();
  if (!user) return null;

  return (
    <header className="navbar">
      <span className="brand">Task Manager</span>
      <nav>
        {user.role === 'admin' && <NavLink to="/admin">Admin</NavLink>}
        <NavLink to="/my-tasks">My tasks</NavLink>
      </nav>
      <div className="nav-user">
        <span>
          {user.full_name} <span className={`badge role-${user.role}`}>{user.role}</span>
        </span>
        <button
          type="button"
          className="btn btn-secondary"
          onClick={() => {
            signOut();
            navigate('/login');
          }}
        >
          Sign out
        </button>
      </div>
    </header>
  );
}
