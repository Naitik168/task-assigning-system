import { apiRequest } from './client';
import type {
  AssignTasksResponse,
  CreateTaskInput,
  MyTasksResponse,
  Role,
  Task,
  TaskPriority,
  TaskStatus,
  User,
} from './types';

export const tasksApi = {
  create: (input: CreateTaskInput) =>
    apiRequest<Task>('/tasks', { method: 'POST', body: input }),

  listAll: () => apiRequest<Task[]>('/tasks'),

  assign: (taskIds: string[], assigneeEmail: string) =>
    apiRequest<AssignTasksResponse>('/tasks/assign', {
      method: 'POST',
      body: { task_ids: taskIds, assignee_email: assigneeEmail },
    }),

  update: (
    id: string,
    changes: Partial<{ title: string; description: string; status: TaskStatus; priority: TaskPriority }>,
  ) => apiRequest<Task>(`/tasks/${id}`, { method: 'PATCH', body: changes }),

  viewMyTasks: () => apiRequest<MyTasksResponse>('/tasks/view-my-tasks'),
};

export const usersApi = {
  list: (role?: Role) => apiRequest<User[]>(role ? `/users?role=${role}` : '/users'),
};
