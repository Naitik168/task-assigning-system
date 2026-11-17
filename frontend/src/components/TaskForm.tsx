import { useState } from 'react';
import type { FormEvent } from 'react';
import type { CreateTaskInput, TaskPriority } from '../api/types';

interface TaskFormProps {
  /** Resolve to reset the form; reject to keep the input for correction. */
  onSubmit: (input: CreateTaskInput) => Promise<void>;
  submitting: boolean;
  submitLabel?: string;
}

export function TaskForm({ onSubmit, submitting, submitLabel = 'Create task' }: TaskFormProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState<TaskPriority>('medium');

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    try {
      await onSubmit({ title: title.trim(), description: description.trim(), priority });
      setTitle('');
      setDescription('');
      setPriority('medium');
    } catch {
      // The parent renders the error; keep the user's input.
    }
  }

  return (
    <form className="form-grid" onSubmit={handleSubmit}>
      <label>
        Title
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
          maxLength={200}
          placeholder="e.g. Infiltrate SPECTRE headquarters"
        />
      </label>
      <label>
        Priority
        <select value={priority} onChange={(e) => setPriority(e.target.value as TaskPriority)}>
          <option value="high">High</option>
          <option value="medium">Medium</option>
          <option value="low">Low</option>
        </select>
      </label>
      <label className="span-2">
        Description
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={2}
          maxLength={2000}
          placeholder="Optional details"
        />
      </label>
      <div className="span-2">
        <button type="submit" className="btn btn-primary" disabled={submitting || !title.trim()}>
          {submitting ? 'Saving…' : submitLabel}
        </button>
      </div>
    </form>
  );
}
