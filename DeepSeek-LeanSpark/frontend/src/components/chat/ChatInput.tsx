// 聊天输入框：含发送/清空按钮。Enter 发送，Shift+Enter 换行。
import { useState } from 'react';

interface Props {
  onSend: (text: string) => void;
  onReset: () => void;
  disabled: boolean;
  placeholder?: string;
}

export function ChatInput({
  onSend,
  onReset,
  disabled,
  placeholder = '输入你的数学命题或问题... (Enter 发送，Shift+Enter 换行)',
}: Props) {
  const [value, setValue] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!value.trim() || disabled) return;
    onSend(value);
    setValue('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!value.trim() || disabled) return;
      onSend(value);
      setValue('');
    }
  };

  return (
    <form
      className="chat-input-form"
      onSubmit={handleSubmit}
      style={{
        display: 'flex',
        gap: 8,
        padding: 12,
        borderTop: '1px solid #eee',
        background: '#fff',
      }}
    >
      <textarea
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={2}
        style={{
          flex: 1,
          padding: '8px 10px',
          border: '1px solid #ddd',
          borderRadius: 6,
          resize: 'vertical',
          fontFamily: 'inherit',
          fontSize: 14,
          lineHeight: 1.5,
          outline: 'none',
          minHeight: 36,
          maxHeight: 200,
        }}
      />
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        <button
          type="submit"
          disabled={disabled || !value.trim()}
          style={{
            padding: '8px 16px',
            background: disabled || !value.trim() ? '#ccc' : '#2563eb',
            color: '#fff',
            border: 'none',
            borderRadius: 6,
            cursor: disabled || !value.trim() ? 'not-allowed' : 'pointer',
            fontSize: 13,
          }}
        >
          发送
        </button>
        <button
          type="button"
          onClick={onReset}
          disabled={disabled}
          style={{
            padding: '6px 12px',
            background: '#f5f5f5',
            color: '#555',
            border: '1px solid #e5e5e5',
            borderRadius: 6,
            cursor: disabled ? 'not-allowed' : 'pointer',
            fontSize: 12,
          }}
        >
          清空
        </button>
      </div>
    </form>
  );
}
