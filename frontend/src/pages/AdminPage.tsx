import { useEffect, useState } from 'react';
import { errorMessage } from '../api/client';
import { tasksApi, usersApi } from '../api/tasks';
import type { CreateTaskInput } from '../api/types';
import { Alert } from '../components/Alert';
import { EmptyState } from '../components/EmptyState';
import { Spinner } from '../components/Spinner';
import { TaskForm } from '../components/TaskForm';
import { TaskTable } from '../components/TaskTable';
import { useAsyncData } from '../hooks/useAsyncData';

export function AdminPage() {
  const tasks = useAsyncData(tasksApi.listAll);
  const staff = useAsyncData(() => usersApi.list('staff'));

  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [createSuccess, setCreateSuccess] = useState<string | null>(null);

  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [assignee, setAssignee] = useState('');
  const [assigning, setAssigning] = useState(false);
  const [assignError, setAssignError] = useState<string | null>(null);
  const [assignSuccess, setAssignSuccess] = useState<string | null>(null);

  // Default the assignee picker to the first staff member (James Bond).
  useEffect(() => {
    if (!assignee && staff.data && staff.data.length > 0) setAssignee(staff.data[0].email);
  }, [staff.data, assignee]);

  async function handleCreate(input: CreateTaskInput) {
    setCreating(true);
    setCreateError(null);
    setCreateSuccess(null);
    try {
      const task = await tasksApi.create(input);
      setCreateSuccess(`Created “${task.title}”.`);
      await tasks.reload();
    } catch (e) {
      setCreateError(errorMessage(e));
      throw e;
    } finally {
      setCreating(false);
    }
  }

  async function handleAssign() {
    setAssigning(true);
    setAssignError(null);
    setAssignSuccess(null);
    try {
      const res = await tasksApi.assign([...selected], assignee);
      setAssignSuccess(`Assigned ${res.assigned_count} task(s) to ${res.assignee.email}.`);
      setSelected(new Set());
      await tasks.reload();
    } catch (e) {
      setAssignError(errorMessage(e));
    } finally {
      setAssigning(false);
    }
  }

  function toggle(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  const taskList = tasks.data ?? [];

  return (
    <main className="page">
      <section className="card">
        <h2>Create task</h2>
        {createError && (
          <Alert kind="error" onDismiss={() => setCreateError(null)}>
            {createError}
          </Alert>
        )}
        {createSuccess && <Alert kind="success">{createSuccess}</Alert>}
        <TaskForm onSubmit={handleCreate} submitting={creating} />
      </section>

      <section className="card">
        <div className="card-header">
          <h2>All tasks {tasks.data && <span className="muted">({taskList.length})</span>}</h2>
          <button type="button" className="btn btn-secondary" onClick={() => tasks.reload()}>
            Refresh
          </button>
        </div>

        {assignError && (
          <Alert kind="error" onDismiss={() => setAssignError(null)}>
            {assignError}
          </Alert>
        )}
        {assignSuccess && <Alert kind="success">{assignSuccess}</Alert>}

        {tasks.loading && !tasks.data ? (
          <Spinner label="Loading tasks…" />
        ) : tasks.error ? (
          <Alert kind="error">{tasks.error}</Alert>
        ) : taskList.length === 0 ? (
          <EmptyState title="No tasks yet">Create your first task above.</EmptyState>
        ) : (
          <>
            <TaskTable
              tasks={taskList}
              selection={{
                selectedIds: selected,
                onToggle: toggle,
                onToggleAll: (checked) =>
                  setSelected(checked ? new Set(taskList.map((t) => t.id)) : new Set()),
              }}
            />
            <div className="assign-bar">
              <span>
                <strong>{selected.size}</strong> selected
              </span>
              <label className="inline">
                Assign to
                {staff.loading ? (
                  <Spinner label="Loading staff…" />
                ) : staff.error ? (
                  <span className="error-text">{staff.error}</span>
                ) : (
                  <select value={assignee} onChange={(e) => setAssignee(e.target.value)}>
                    {(staff.data ?? []).map((u) => (
                      <option key={u.id} value={u.email}>
                        {u.full_name} ({u.email})
                      </option>
                    ))}
                  </select>
                )}
              </label>
              <button
                type="button"
                className="btn btn-primary"
                disabled={selected.size === 0 || !assignee || assigning}
                onClick={handleAssign}
              >
                {assigning ? 'Assigning…' : 'Assign selected'}
              </button>
            </div>
          </>
        )}
      </section>
    </main>
  );
}
