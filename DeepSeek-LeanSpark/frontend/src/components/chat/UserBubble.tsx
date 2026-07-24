// 用户气泡：靠右、灰色圆角、最大宽度 60%（Trae Work 风格）、不显示"你"字。
// 纯文本，不解析 Markdown；气泡宽度自适应内容，不占满整行。
interface Props {
  text: string;
}

export function UserBubble({ text }: Props) {
  return (
    <div className="msg-row user-row" style={{ display: 'flex', justifyContent: 'flex-end' }}>
      <div
        className="user-bubble"
        style={{
          maxWidth: '60%',
          background: '#f4f4f5',
          borderRadius: 10,
          padding: '8px 12px',
          color: '#1f2937',
          fontSize: 14,
          lineHeight: 1.5,
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
        }}
      >
        {text}
      </div>
    </div>
  );
}
