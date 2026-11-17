import { useState } from 'react';
import { ApiError, errorMessage } from '../api/client';
import { tasksApi } from '../api/tasks';
import type { CreateTaskInput } from '../api/types';
import { Alert } from '../components/Alert';
import { CacheBadge } from '../components/Badges';
import { EmptyState } from '../components/EmptyState';
import { Spinner } from '../components/Spinner';
import { TaskForm } from '../components/TaskForm';
import { TaskTable } from '../components/TaskTable';
import { useAuth } from '../context/AuthContext';
import { useAsyncData } from '../hooks/useAsyncData';

export function MyTasksPage() {
  const { user } = useAuth();
  const myTasks = useAsyncData(tasksApi.viewMyTasks);

  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<{ forbidden: boolean; message: string } | null>(null);
  const [createSuccess, setCreateSuccess] = useState<string | null>(null);

  async function attemptCreate(input: CreateTaskInput) {
    setCreating(true);
    setCreateError(null);
    setCreateSuccess(null);
    try {
      const task = await tasksApi.create(input);
      setCreateSuccess(`Created “${task.title}”.`);
    } catch (e) {
      const forbidden = e instanceof ApiError && e.isForbidden;
      setCreateError({ forbidden, message: errorMessage(e) });
      throw e;
    } finally {
      setCreating(false);
    }
  }

  const data = myTasks.data;

  return (
    <main className="page">
      <section className="card">
        <div className="card-header">
          <div>
            <h2>My assigned tasks</h2>
            {data && (
              <p className="muted">
                {data.user.email} · role <strong>{data.user.role}</strong> ·{' '}
                <span data-testid="task-total">{data.summary.total_assigned_tasks}</span> assigned
              </p>
            )}
          </div>
          <div className="row">
            {data && <CacheBadge hit={data.cache.hit} />}
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => myTasks.reload()}
              disabled={myTasks.loading}
            >
              {myTasks.loading && data ? 'Refreshing…' : 'Refresh'}
            </button>
          </div>
        </div>

        {myTasks.loading && !data ? (
          <Spinner label="Loading your tasks…" />
        ) : myTasks.error ? (
          <Alert kind="error" title="Could not load tasks">
            {myTasks.error}
          </Alert>
        ) : !data || data.tasks.length === 0 ? (
          <EmptyState title="No tasks assigned to you yet">
            An admin can assign tasks to you from the Admin screen.
          </EmptyState>
        ) : (
          <TaskTable tasks={data.tasks} />
        )}
      </section>

      {user?.role === 'staff' && (
        <section className="card">
          <h2>Create a task</h2>
          <p className="muted small">
            Staff accounts are not allowed to create tasks. Try it to see how the API enforces this.
          </p>
          {createError &&
            (createError.forbidden ? (
              <Alert kind="warning" title="Access denied (403 Forbidden)" onDismiss={() => setCreateError(null)}>
                You are signed in as staff, and only admins can create tasks. The server said: “
                {createError.message}”
              </Alert>
            ) : (
              <Alert kind="error" onDismiss={() => setCreateError(null)}>
                {createError.message}
              </Alert>
            ))}
          {createSuccess && <Alert kind="success">{createSuccess}</Alert>}
          <TaskForm onSubmit={attemptCreate} submitting={creating} submitLabel="Try to create task" />
        </section>
      )}
    </main>
  );
}
