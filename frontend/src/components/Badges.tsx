import type { TaskPriority, TaskStatus } from '../api/types';

const STATUS_LABEL: Record<TaskStatus, string> = {
  todo: 'To do',
  in_progress: 'In progress',
  done: 'Done',
};

export function PriorityBadge({ priority }: { priority: TaskPriority }) {
  return <span className={`badge priority-${priority}`}>{priority}</span>;
}

export function StatusBadge({ status }: { status: TaskStatus }) {
  return <span className={`badge status-${status}`}>{STATUS_LABEL[status]}</span>;
}

export function CacheBadge({ hit }: { hit: boolean }) {
  return (
    <span className={`badge ${hit ? 'cache-hit' : 'cache-miss'}`} data-testid="cache-badge">
      {hit ? 'cache.hit = true (served from cache)' : 'cache.hit = false (loaded from database)'}
    </span>
  );
}
