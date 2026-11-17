// Types mirroring the Rust API DTOs.

export type Role = 'admin' | 'staff';
export type TaskStatus = 'todo' | 'in_progress' | 'done';
export type TaskPriority = 'low' | 'medium' | 'high';

export interface User {
  id: string;
  full_name: string;
  email: string;
  role: Role;
  created_at: string;
}

export interface Task {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  priority: TaskPriority;
  created_by: string;
  assigned_to: string | null;
  created_at: string;
  updated_at: string;
}

export interface LoginResponse {
  login_challenge_id: string;
  message: string;
  expires_in_seconds: number;
}

export interface TokenResponse {
  access_token: string;
  token_type: string;
  expires_in_seconds: number;
  user: User;
}

export interface UserSummary {
  email: string;
  role: Role;
}

export interface MyTasksResponse {
  user: UserSummary;
  tasks: Task[];
  summary: { total_assigned_tasks: number };
  cache: { hit: boolean };
}

export interface AssignTasksResponse {
  assignee: UserSummary;
  assigned_count: number;
  tasks: Task[];
}

export interface DevEmail {
  id: string;
  to_email: string;
  subject: string;
  body: string;
  verification_code: string | null;
  created_at: string;
}

export interface SeedUsersResponse {
  users: Array<User & { created: boolean }>;
}

export interface CreateTaskInput {
  title: string;
  description?: string;
  priority?: TaskPriority;
  status?: TaskStatus;
}

export interface ApiErrorBody {
  error: { code: string; message: string };
}
