// 证明依赖图 Tab：从最近一次 run_lean_check 提交的代码生成依赖图。
// 顶部提供"刷新"按钮，可手动重新分析。
import type { UseProofGraphResult } from '../../hooks/useProofGraph';
import { ProofGraphView } from './ProofGraphView';

interface Props {
  proofGraph: UseProofGraphResult;
  /** 最近一次 run_lean_check 提交的代码（自动触发分析） */
  lastLeanCode: string | null;
}

export function ProofGraphTab({ proofGraph, lastLeanCode }: Props) {
  const { graph, loading, error, fetchGraph } = proofGraph;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '4px 8px',
          borderBottom: '1px solid #eee',
          background: '#fafafa',
          fontSize: 12,
        }}
      >
        <span style={{ color: '#555' }}>
          {lastLeanCode
            ? `已分析 ${graph?.nodes.length ?? 0} 节点 / ${graph?.edges.length ?? 0} 边`
            : '尚无 Lean4 代码'}
        </span>
        <button
          type="button"
          onClick={() => lastLeanCode && fetchGraph(lastLeanCode)}
          disabled={!lastLeanCode || loading}
          title="重新分析"
          style={{
            border: '1px solid #e5e5e5',
            background: '#fff',
            borderRadius: 3,
            padding: '2px 8px',
            fontSize: 11,
            cursor: !lastLeanCode || loading ? 'not-allowed' : 'pointer',
            opacity: !lastLeanCode || loading ? 0.5 : 1,
          }}
        >
          {loading ? '分析中...' : '⟳ 刷新'}
        </button>
      </div>
      {error && (
        <div
          style={{
            padding: '4px 8px',
            color: '#dc2626',
            fontSize: 11,
            background: '#fef2f2',
          }}
        >
          {error}
        </div>
      )}
      <div style={{ flex: 1, position: 'relative' }}>
        <ProofGraphView graph={graph} />
      </div>
    </div>
  );
}
