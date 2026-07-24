// 消息容器：根据 role 分发到 UserBubble 或 AssistantMessage。
import type { ChatMessage } from '../../types';
import { UserBubble } from './UserBubble';
import { AssistantMessage } from './AssistantMessage';

interface Props {
  message: ChatMessage;
  onFormulaClick?: (latex: string) => void;
}

export function MessageBubble({ message, onFormulaClick }: Props) {
  if (message.role === 'user') {
    return <UserBubble text={message.content} />;
  }
  return <AssistantMessage message={message} onFormulaClick={onFormulaClick} />;
}
