-- Tasks. Enum order matters: ORDER BY priority DESC yields high, medium, low.
CREATE TYPE task_status AS ENUM ('todo', 'in_progress', 'done');
CREATE TYPE task_priority AS ENUM ('low', 'medium', 'high');

CREATE TABLE tasks (
    id             UUID PRIMARY KEY,
    title          TEXT          NOT NULL CHECK (length(trim(title)) > 0),
    description    TEXT          NOT NULL DEFAULT '',
    status         task_status   NOT NULL DEFAULT 'todo',
    priority       task_priority NOT NULL DEFAULT 'medium',
    created_by_id  UUID          NOT NULL REFERENCES users (id),
    assigned_to_id UUID          REFERENCES users (id) ON DELETE SET NULL,
    created_at     TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_tasks_assigned_to_id ON tasks (assigned_to_id);
CREATE INDEX idx_tasks_created_by_id ON tasks (created_by_id);
