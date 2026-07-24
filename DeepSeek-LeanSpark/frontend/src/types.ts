// Agent 循环产生的事件，与后端 `AgentEvent`（src/agent/agent_loop.rs）一一对应。
export interface AgentEvent {
  kind: 'thinking' | 'tool_call' | 'tool_result' | 'answer' | 'error';
  content: string;
  tool_name?: string;
  tool_args?: unknown;
  // 被 write_file 修改的相对路径（仅 tool_result 事件填）
  files_changed?: string[];
  // 被 write_file 创建的新文件相对路径（仅 tool_result 事件填）
  files_created?: string[];
  // 整轮 Agent 循环耗时（毫秒），仅 answer/error 事件填
  duration_ms?: number;
}

export interface ChatResponseDto {
  events: AgentEvent[];
  messages: unknown[];
}

export interface LeanResult {
  success: boolean;
  output: string;
  error: string | null;
  contains_sorry: boolean;
}

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  events?: AgentEvent[];
  // 整轮耗时（毫秒），仅 assistant 消息填（取自 answer 事件）
  duration_ms?: number;
  // 整轮累计文件变更（取自所有 tool_result 累加）
  files_changed?: string[];
  files_created?: string[];
}

// ============ 工作区 ============

export interface FileNode {
  name: string;
  path: string; // 相对工作区根的 POSIX 路径
  kind: 'file' | 'dir';
  size?: number | null;
  children?: FileNode[] | null;
}

export interface WorkspaceCurrentDto {
  open: boolean;
  path?: string | null;
  tree?: FileNode | null;
}

export interface WorkspaceReadDto {
  success: boolean;
  path: string;
  content?: string | null;
  error?: string | null;
}

export interface WorkspaceWriteDto {
  success: boolean;
  path: string;
  created?: boolean;
  bytes?: number;
  error?: string;
}

// ============ 证明依赖图 ============

export interface GraphNode {
  id: string;
  name: string;
  kind: 'theorem' | 'lemma' | 'external';
  external: boolean;
}

export interface GraphEdge {
  from: string;
  to: string;
}

export interface ProofGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  note: string;
}
