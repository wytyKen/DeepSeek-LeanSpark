// 右侧栏容器：上方三 Tab 切换（资源管理器 / 证明依赖图 / 公式），下方对应内容。
import { useState } from 'react';
import type { ChatMessage } from '../../types';
import type { UseWorkspaceResult } from '../../hooks/useWorkspace';
import type { UseProofGraphResult } from '../../hooks/useProofGraph';
import { ExplorerTab } from './ExplorerTab';
import { ProofGraphTab } from './ProofGraphTab';
import { FormulaTab } from './FormulaTab';

type TabKey = 'explorer' | 'proof-graph' | 'formula';

interface Props {
  workspace: UseWorkspaceResult;
  proofGraph: UseProofGraphResult;
  messages: ChatMessage[];
  selectedFile: string | null;
  onOpenFile: (relPath: string) => void;
  onFormulaClick?: (latex: string) => void;
}

const TABS: { key: TabKey; label: string }[] = [
  { key: 'explorer', label: '资源管理器' },
  { key: 'proof-graph', label: '证明依赖图' },
  { key: 'formula', label: '公式' },
];

export function RightSidebar({
  workspace,
  proofGraph,
  messages,
  selectedFile,
  onOpenFile,
  onFormulaClick,
}: Props) {
  const [tab, setTab] = useState<TabKey>('explorer');

  return (
    <div
      className="right-sidebar"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        background: '#fff',
        borderLeft: '1px solid #ddd',
      }}
    >
      {/* Tab 头 */}
      <div
        className="sidebar-tabs"
        style={{
          display: 'flex',
          borderBottom: '1px solid #ddd',
          background: '#fafafa',
        }}
      >
        {TABS.map((t) => (
          <button
            key={t.key}
            type="button"
            onClick={() => setTab(t.key)}
            style={{
              flex: 1,
              padding: '8px 12px',
              fontSize: 12,
              background: tab === t.key ? '#fff' : 'transparent',
              color: tab === t.key ? '#2563eb' : '#555',
              border: 'none',
              borderBottom: tab === t.key ? '2px solid #2563eb' : '2px solid transparent',
              cursor: 'pointer',
              fontWeight: tab === t.key ? 500 : 400,
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab 内容 */}
      <div style={{ flex: 1, overflow: 'hidden' }}>
        {tab === 'explorer' && (
          <ExplorerTab
            workspace={workspace}
            selectedPath={selectedFile}
            onOpenFile={onOpenFile}
          />
        )}
        {tab === 'proof-graph' && (
          <ProofGraphTab
            proofGraph={proofGraph}
            lastLeanCode={messages.length > 0 ? extractLastLeanCode(messages) : null}
          />
        )}
        {tab === 'formula' && (
          <FormulaTab messages={messages} onFormulaClick={onFormulaClick} />
        )}
      </div>
    </div>
  );
}

// 从最近一次 assistant 消息中提取 run_lean_check 的代码参数
function extractLastLeanCode(messages: ChatMessage[]): string | null {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (m.role !== 'assistant') continue;
    for (let j = (m.events ?? []).length - 1; j >= 0; j--) {
      const ev = m.events![j];
      if (ev.kind === 'tool_call' && ev.tool_name === 'run_lean_check') {
        const args = ev.tool_args as { lean_code?: string } | null;
        if (args?.lean_code) return args.lean_code;
      }
    }
  }
  return null;
}
