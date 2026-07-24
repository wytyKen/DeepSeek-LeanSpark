// 复制按钮：点击把指定文本写入剪贴板，给出"已复制"反馈。
import { useState } from 'react';

interface Props {
  text: string;
  label?: string;
  size?: 'sm' | 'md';
}

export function CopyButton({ text, label = '复制', size = 'sm' }: Props) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 降级方案：用 textarea + execCommand
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand('copy');
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      } catch {
        // 静默失败
      }
      document.body.removeChild(ta);
    }
  };

  const fontSize = size === 'sm' ? 11 : 13;
  return (
    <button
      type="button"
      onClick={handleCopy}
      className="copy-btn"
      style={{
        fontSize,
        padding: size === 'sm' ? '2px 6px' : '4px 10px',
        background: copied ? '#dcfce7' : '#f5f5f5',
        color: copied ? '#15803d' : '#555',
        border: '1px solid #e5e5e5',
        borderRadius: 4,
        cursor: 'pointer',
      }}
      aria-label={label}
    >
      {copied ? '已复制' : label}
    </button>
  );
}
