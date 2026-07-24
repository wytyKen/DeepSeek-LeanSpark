// 证明依赖图数据获取：把 Lean4 代码 POST 到 /api/proof-graph，返回节点+边数据。
import { useCallback, useState } from 'react';
import type { ProofGraph } from '../types';

export interface UseProofGraphResult {
  graph: ProofGraph | null;
  loading: boolean;
  error: string | null;
  fetchGraph: (code: string) => Promise<ProofGraph | null>;
  clear: () => void;
}

export function useProofGraph(): UseProofGraphResult {
  const [graph, setGraph] = useState<ProofGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchGraph = useCallback(async (code: string) => {
    if (!code.trim()) {
      setGraph(null);
      return null;
    }
    setLoading(true);
    setError(null);
    try {
      const resp = await fetch('/api/proof-graph', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code }),
      });
      if (!resp.ok) {
        const text = await resp.text();
        throw new Error(`HTTP ${resp.status}: ${text}`);
      }
      const data: ProofGraph = await resp.json();
      setGraph(data);
      return data;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  const clear = useCallback(() => {
    setGraph(null);
    setError(null);
  }, []);

  return { graph, loading, error, fetchGraph, clear };
}
