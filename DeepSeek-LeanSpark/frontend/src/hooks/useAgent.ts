// Agent 调用 hook：管理聊天消息历史，调用 /api/chat，从事件中汇总耗时与文件变更。
import { useCallback, useState } from 'react';
import type { AgentEvent, ChatMessage, ChatResponseDto } from '../types';

interface UseAgentOptions {
  thinking?: boolean;
}

export function useAgent(options: UseAgentOptions = {}) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const send = useCallback(
    async (text: string) => {
      if (!text.trim() || isRunning) return;
      setIsRunning(true);
      setError(null);

      // 构造给后端的历史：把本地消息序列扁平化为 {role, content}
      const history = messages.map((m) => ({ role: m.role, content: m.content }));

      // 乐观追加用户消息
      const userMsg: ChatMessage = { role: 'user', content: text };
      setMessages((prev) => [...prev, userMsg]);

      try {
        const resp = await fetch('/api/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            message: text,
            history,
            thinking: options.thinking ?? false,
          }),
        });
        if (!resp.ok) {
          const errText = await resp.text();
          throw new Error(`HTTP ${resp.status}: ${errText}`);
        }
        const data: ChatResponseDto = await resp.json();

        // 从事件中抽取最终的 answer 作为 assistant 消息内容
        const answerEvent = [...data.events].reverse().find((e) => e.kind === 'answer');
        const errorEvent = [...data.events].reverse().find((e) => e.kind === 'error');
        const finalEvent = answerEvent ?? errorEvent;
        const assistantContent = finalEvent?.content ?? '(无回答)';

        // 累加所有 tool_result 事件的文件变更
        const filesChanged: string[] = [];
        const filesCreated: string[] = [];
        for (const ev of data.events) {
          if (ev.kind === 'tool_result') {
            for (const p of ev.files_changed ?? []) {
              if (!filesChanged.includes(p)) filesChanged.push(p);
            }
            for (const p of ev.files_created ?? []) {
              if (!filesCreated.includes(p)) filesCreated.push(p);
            }
          }
        }

        const assistantMsg: ChatMessage = {
          role: 'assistant',
          content: assistantContent,
          events: data.events,
          duration_ms: finalEvent?.duration_ms,
          files_changed: filesChanged.length > 0 ? filesChanged : undefined,
          files_created: filesCreated.length > 0 ? filesCreated : undefined,
        };
        setMessages((prev) => [...prev, assistantMsg]);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        setMessages((prev) => [
          ...prev,
          { role: 'assistant', content: `错误: ${msg}` },
        ]);
      } finally {
        setIsRunning(false);
      }
    },
    [messages, isRunning, options.thinking],
  );

  const reset = useCallback(() => {
    setMessages([]);
    setError(null);
  }, []);

  return { messages, send, reset, isRunning, error };
}

// 提取最近一次 assistant 消息中 run_lean_check 提交的 Lean4 代码（供证明依赖图使用）
export function lastLeanCodeFromMessages(messages: ChatMessage[]): string | null {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (m.role !== 'assistant') continue;
    for (let j = (m.events ?? []).length - 1; j >= 0; j--) {
      const ev: AgentEvent = m.events![j];
      if (ev.kind === 'tool_call' && ev.tool_name === 'run_lean_check') {
        const args = ev.tool_args as { lean_code?: string } | null;
        if (args?.lean_code) return args.lean_code;
      }
    }
  }
  return null;
}

// 提取当前会话所有 assistant 回答中的块级 LaTeX 公式（$$...$$）
export function collectBlockLatex(messages: ChatMessage[]): string[] {
  const out: string[] = [];
  const re = /\$\$([\s\S]+?)\$\$/g;
  for (const m of messages) {
    if (m.role !== 'assistant') continue;
    let match: RegExpExecArray | null;
    while ((match = re.exec(m.content)) !== null) {
      out.push(match[1].trim());
    }
  }
  return out;
}
