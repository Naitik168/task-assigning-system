import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { bondUser, jsonResponse, renderApp } from '../test/utils';

const task = (id: string, title: string, priority: string) => ({
  id,
  title,
  description: '',
  status: 'todo',
  priority,
  created_by: 'admin@example.com',
  assigned_to: bondUser.email,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
});

const response = (hit: boolean, tasks = [task('1', 'Infiltrate SPECTRE', 'high'), task('2', 'Recover Lektor', 'medium'), task('3', 'Debrief M', 'low')]) => ({
  user: { email: bondUser.email, role: 'staff' },
  tasks,
  summary: { total_assigned_tasks: tasks.length },
  cache: { hit },
});

describe('MyTasksPage', () => {
  afterEach(() => vi.restoreAllMocks());

  it('renders API-driven tasks and the cache status, then a cache hit on refresh', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(jsonResponse(response(false)))
      .mockResolvedValueOnce(jsonResponse(response(true)));
    renderApp('/my-tasks', { user: bondUser, token: 'jwt-bond' });

    expect(screen.getByText('Loading your tasks…')).toBeInTheDocument();
    expect(await screen.findAllByTestId('task-row')).toHaveLength(3);
    expect(screen.getByTestId('task-total')).toHaveTextContent('3');
    expect(screen.getByTestId('cache-badge')).toHaveTextContent('cache.hit = false');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect((fetchMock.mock.calls[0][1]?.headers as Record<string, string>).Authorization).toBe('Bearer jwt-bond');

    await userEvent.click(screen.getByRole('button', { name: 'Refresh' }));
    expect(await screen.findByText(/cache.hit = true/)).toBeInTheDocument();
  });

  it('shows the empty state', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(jsonResponse(response(false, [])));
    renderApp('/my-tasks', { user: bondUser });
    expect(await screen.findByText('No tasks assigned to you yet')).toBeInTheDocument();
  });

  it('shows an error state when the API fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      jsonResponse({ error: { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred.' } }, 500),
    );
    renderApp('/my-tasks', { user: bondUser });
    const alert = await screen.findByRole('alert');
    expect(within(alert).getByText('Could not load tasks')).toBeInTheDocument();
  });

  it('shows a clear message when staff are blocked from creating a task', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      if (String(input).endsWith('/tasks') && init?.method === 'POST')
        return jsonResponse({ error: { code: 'FORBIDDEN', message: 'Only admins can perform this action.' } }, 403);
      return jsonResponse(response(false));
    });
    const user = userEvent.setup();
    renderApp('/my-tasks', { user: bondUser });

    await screen.findAllByTestId('task-row');
    await user.type(screen.getByLabelText('Title'), 'Rogue mission');
    await user.click(screen.getByRole('button', { name: 'Try to create task' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Access denied (403 Forbidden)');
    expect(alert).toHaveTextContent('only admins can create tasks');
  });
});
