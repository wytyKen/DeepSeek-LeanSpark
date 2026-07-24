// 公式 Tab：收集当前会话所有 assistant 回答中的块级 LaTeX 公式，列表展示。
import type { ChatMessage } from '../../types';
import { FormulaCard } from './FormulaCard';

interface Props {
  messages: ChatMessage[];
  onFormulaClick?: (latex: string) => void;
}

interface FormulaEntry {
  latex: string;
  source: string;
}

export function FormulaTab({ messages, onFormulaClick }: Props) {
  const formulas = collectFormulas(messages);

  if (formulas.length === 0) {
    return (
      <div
        style={{
          padding: 16,
          color: '#888',
          fontSize: 13,
          textAlign: 'center',
        }}
      >
        尚无公式
        <div style={{ fontSize: 11, color: '#aaa', marginTop: 4 }}>
          Agent 在回答中使用 LaTeX 块级公式时，将在此处自动收集
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        padding: 8,
        overflowY: 'auto',
        height: '100%',
      }}
    >
      <div
        style={{
          fontSize: 11,
          color: '#888',
          marginBottom: 8,
          padding: '0 4px',
        }}
      >
        共 {formulas.length} 个公式
      </div>
      {formulas.map((f, i) => (
        <FormulaCard
          key={`${i}-${f.latex.slice(0, 20)}`}
          latex={f.latex}
          source={f.source}
          onClick={onFormulaClick}
        />
      ))}
    </div>
  );
}

function collectFormulas(messages: ChatMessage[]): FormulaEntry[] {
  const out: FormulaEntry[] = [];
  const re = /\$\$([\s\S]+?)\$\$/g;
  // 找出每条 assistant 消息的轮次序号（按 assistant 出现顺序计数）
  let assistantIdx = 0;
  for (const m of messages) {
    if (m.role !== 'assistant') continue;
    assistantIdx += 1;
    let match: RegExpExecArray | null;
    while ((match = re.exec(m.content)) !== null) {
      const latex = match[1].trim();
      if (latex) {
        out.push({ latex, source: `来自第 ${assistantIdx} 轮回答` });
      }
    }
  }
  return out;
}
