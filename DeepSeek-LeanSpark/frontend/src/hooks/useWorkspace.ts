// 工作区状态管理：跟踪当前打开的工作区路径与文件树，提供打开/关闭/读写能力。
import { useCallback, useEffect, useState } from 'react';
import type {
  FileNode,
  WorkspaceCurrentDto,
  WorkspaceReadDto,
  WorkspaceWriteDto,
} from '../types';

export interface UseWorkspaceResult {
  open: boolean;
  path: string | null;
  tree: FileNode | null;
  loading: boolean;
  error: string | null;
  openWorkspace: (path: string) => Promise<boolean>;
  closeWorkspace: () => Promise<void>;
  refreshTree: () => Promise<void>;
  readFile: (relPath: string) => Promise<WorkspaceReadDto>;
  writeFile: (relPath: string, content: string) => Promise<WorkspaceWriteDto>;
}

export function useWorkspace(): UseWorkspaceResult {
  const [open, setOpen] = useState(false);
  const [path, setPath] = useState<string | null>(null);
  const [tree, setTree] = useState<FileNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 启动时拉取一次当前工作区状态（应对后端已持久化场景）
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const resp = await fetch('/api/workspace/current');
        if (!resp.ok) return;
        const data: WorkspaceCurrentDto = await resp.json();
        if (cancelled) return;
        setOpen(data.open);
        setPath(data.path ?? null);
        setTree(data.tree ?? null);
      } catch {
        // 静默：启动时后端可能未就绪
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const openWorkspace = useCallback(async (target: string) => {
    setLoading(true);
    setError(null);
    try {
      const resp = await fetch('/api/workspace/open', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: target }),
      });
      if (!resp.ok) {
        const text = await resp.text();
        throw new Error(`HTTP ${resp.status}: ${text}`);
      }
      const data: WorkspaceCurrentDto = await resp.json();
      setOpen(data.open);
      setPath(data.path ?? null);
      setTree(data.tree ?? null);
      return true;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  const closeWorkspace = useCallback(async () => {
    await fetch('/api/workspace/close', { method: 'POST' });
    setOpen(false);
    setPath(null);
    setTree(null);
  }, []);

  const refreshTree = useCallback(async () => {
    if (!open) return;
    try {
      const resp = await fetch('/api/workspace/tree');
      if (!resp.ok) return;
      const data: FileNode | null = await resp.json();
      setTree(data);
    } catch {
      // 静默
    }
  }, [open]);

  const readFile = useCallback(async (relPath: string) => {
    const resp = await fetch('/api/workspace/read', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: relPath }),
    });
    return (await resp.json()) as WorkspaceReadDto;
  }, []);

  const writeFile = useCallback(async (relPath: string, content: string) => {
    const resp = await fetch('/api/workspace/write', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: relPath, content }),
    });
    return (await resp.json()) as WorkspaceWriteDto;
  }, []);

  return {
    open,
    path,
    tree,
    loading,
    error,
    openWorkspace,
    closeWorkspace,
    refreshTree,
    readFile,
    writeFile,
  };
}
