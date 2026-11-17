import { useCallback, useEffect, useRef, useState } from 'react';
import { errorMessage } from '../api/client';

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  reload: () => Promise<void>;
}

/** Loads data on mount and exposes loading/error state plus a reload function. */
export function useAsyncData<T>(loader: () => Promise<T>): AsyncState<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const loaderRef = useRef(loader);
  loaderRef.current = loader;

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await loaderRef.current());
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Load once on mount. The guard stops React StrictMode's dev-only double
  // effect from issuing two requests (which would e.g. turn the very first
  // view-my-tasks response into a cache hit).
  const startedRef = useRef(false);
  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void reload();
  }, [reload]);

  return { data, loading, error, reload };
}
