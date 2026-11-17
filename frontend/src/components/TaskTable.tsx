import type { Task } from '../api/types';
import { PriorityBadge, StatusBadge } from './Badges';

interface TaskTableProps {
  tasks: Task[];
  /** When provided, a checkbox column is rendered. */
  selection?: {
    selectedIds: Set<string>;
    onToggle: (id: string) => void;
    onToggleAll: (checked: boolean) => void;
  };
}

export function TaskTable({ tasks, selection }: TaskTableProps) {
  const allSelected = selection !== undefined && tasks.length > 0 && tasks.every((t) => selection.selectedIds.has(t.id));

  return (
    <div className="table-wrap">
      <table className="task-table">
        <thead>
          <tr>
            {selection && (
              <th className="col-check">
                <input
                  type="checkbox"
                  aria-label="Select all tasks"
                  checked={allSelected}
                  onChange={(e) => selection.onToggleAll(e.target.checked)}
                />
              </th>
            )}
            <th>Title</th>
            <th>Priority</th>
            <th>Status</th>
            <th>Assigned to</th>
          </tr>
        </thead>
        <tbody>
          {tasks.map((task) => (
            <tr key={task.id} data-testid="task-row">
              {selection && (
                <td className="col-check">
                  <input
                    type="checkbox"
                    aria-label={`Select ${task.title}`}
                    checked={selection.selectedIds.has(task.id)}
                    onChange={() => selection.onToggle(task.id)}
                  />
                </td>
              )}
              <td>
                <div className="task-title">{task.title}</div>
                {task.description && <div className="muted small">{task.description}</div>}
              </td>
              <td>
                <PriorityBadge priority={task.priority} />
              </td>
              <td>
                <StatusBadge status={task.status} />
              </td>
              <td>{task.assigned_to ?? <span className="muted">Unassigned</span>}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
