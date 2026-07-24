// 聊天面板：左侧整体，含消息列表与输入框。Trae Work 风格。
import { useEffect, useRef } from 'react';
import type { ChatMessage } from '../../types';
import { MessageBubble } from './MessageBubble';
import { ChatInput } from './ChatInput';

interface Props {
  messages: ChatMessage[];
  isRunning: boolean;
  onSend: (text: string) => void;
  onReset: () => void;
  onFormulaClick?: (latex: string) => void;
}

export function ChatPanel({
  messages,
  isRunning,
  onSend,
  onReset,
  onFormulaClick,
}: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // 新消息或运行中时自动滚动到底部
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, isRunning]);

  return (
    <div
      className="chat-panel"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      <div
        ref={scrollRef}
        className="chat-messages"
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '16px 12px',
          background: '#fff',
        }}
      >
        {messages.length === 0 && (
          <div
            className="chat-empty"
            style={{
              color: '#999',
              fontSize: 13,
              textAlign: 'center',
              marginTop: 40,
              padding: '0 20px',
            }}
          >
            输入数学命题开始对话。例如：证明 1+1=2，或证明连续函数的中值定理。
            <br />
            打开右侧"资源管理器"可加载工作区，Agent 将可读写工作区内的 .lean 文件。
          </div>
        )}
        {messages.map((m, i) => (
          <div
            key={i}
            className="msg-item"
            style={{ marginBottom: 20, display: 'flex', flexDirection: 'column' }}
          >
            <MessageBubble message={m} onFormulaClick={onFormulaClick} />
          </div>
        ))}
        {isRunning && (
          <div
            className="chat-running"
            style={{ color: '#888', fontSize: 12, padding: '4px 6px' }}
          >
            <span style={{ display: 'inline-block', animation: 'blink 1s infinite' }}>
              ●
            </span>{' '}
            Agent 思考中...
          </div>
        )}
      </div>
      <ChatInput onSend={onSend} onReset={onReset} disabled={isRunning} />
    </div>
  );
}
