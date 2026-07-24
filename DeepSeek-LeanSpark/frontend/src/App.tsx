// DeepSeek-LeanSpark 前端入口。
// 布局：顶部栏 + 主区（左：聊天面板，右：右侧栏三 Tab）。
import { useEffect, useState } from 'react';
import { useAgent, lastLeanCodeFromMessages } from './hooks/useAgent';
import { useWorkspace } from './hooks/useWorkspace';
import { useProofGraph } from './hooks/useProofGraph';
import { ChatPanel } from './components/chat/ChatPanel';
import { RightSidebar } from './components/sidebar/RightSidebar';
import { WorkspaceSwitcher } from './components/workspace/WorkspaceSwitcher';
import { CodeEditor } from './components/workspace/CodeEditor';
import { LatexModal } from './components/common/LatexModal';
import 'katex/dist/katex.min.css';
import './styles/chat.css';

export default function App() {
  const [thinking, setThinking] = useState(false);
  const { messages, send, reset, isRunning } = useAgent({ thinking });
  const workspace = useWorkspace();
  const proofGraph = useProofGraph();

  // 当前在编辑器中打开的文件
  const [openFile, setOpenFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState('');
  const [fileDirty, setFileDirty] = useState(false);
  const [fileLoading, setFileLoading] = useState(false);

  // 当前要放大的 LaTeX 公式
  const [latexModal, setLatexModal] = useState<string | null>(null);

  // 当文件被修改后保存
  const handleSaveFile = async () => {
    if (!openFile) return;
    const result = await workspace.writeFile(openFile, fileContent);
    if (result.success) {
      setFileDirty(false);
      // 保存后刷新文件树
      await workspace.refreshTree();
    } else {
      alert(`保存失败：${result.error ?? '未知错误'}`);
    }
  };

  // 打开文件：从工作区读取
  const handleOpenFile = async (relPath: string) => {
    if (fileDirty && !window.confirm('当前文件未保存，确定切换吗？')) return;
    setFileLoading(true);
    try {
      const result = await workspace.readFile(relPath);
      if (result.success && result.content != null) {
        setOpenFile(relPath);
        setFileContent(result.content);
        setFileDirty(false);
      } else {
        alert(`读取失败：${result.error ?? '未知错误'}`);
      }
    } finally {
      setFileLoading(false);
    }
  };

  // 监听消息变化：当有新的 run_lean_check 调用时自动刷新证明依赖图
  useEffect(() => {
    const code = lastLeanCodeFromMessages(messages);
    if (code) {
      proofGraph.fetchGraph(code);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages]);

  // 当文件被 Agent 修改时，刷新文件树；如果当前打开的文件被改了，也刷新编辑器内容
  useEffect(() => {
    const lastMsg = messages[messages.length - 1];
    if (lastMsg?.role === 'assistant' && (lastMsg.files_changed ?? []).length > 0) {
      workspace.refreshTree();
      const changed = lastMsg.files_changed ?? [];
      if (openFile && changed.includes(openFile)) {
        // 重新加载当前打开的文件
        workspace.readFile(openFile).then((r) => {
          if (r.success && r.content != null) {
            setFileContent(r.content);
            setFileDirty(false);
          }
        });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages, openFile]);

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header className="app-header">
        <h1>DeepSeek-LeanSpark</h1>
        <span className="header-subtitle">
          DeepSeek V4 + Lean4 形式化证明助手
        </span>
        <label className="header-thinking">
          <input
            type="checkbox"
            checked={thinking}
            onChange={(e) => setThinking(e.target.checked)}
          />{' '}
          thinking 模式
        </label>
        <WorkspaceSwitcher workspace={workspace} />
      </header>

      <main className="app-main">
        {/* 左侧：聊天面板 */}
        <div className="left-pane">
          <ChatPanel
            messages={messages}
            isRunning={isRunning}
            onSend={send}
            onReset={reset}
            onFormulaClick={(latex) => setLatexModal(latex)}
          />
        </div>

        {/* 右侧：上方 Tab + 下方编辑器 */}
        <div className="right-pane">
          <div style={{ flex: 1, overflow: 'hidden', minHeight: 0 }}>
            <RightSidebar
              workspace={workspace}
              proofGraph={proofGraph}
              messages={messages}
              selectedFile={openFile}
              onOpenFile={handleOpenFile}
              onFormulaClick={(latex) => setLatexModal(latex)}
            />
          </div>
          <div className="editor-pane" style={{ padding: 6 }}>
            {openFile ? (
              <CodeEditor
                filePath={openFile}
                value={fileContent}
                onChange={(v) => {
                  setFileContent(v);
                  setFileDirty(true);
                }}
                onSave={handleSaveFile}
                readOnly={fileLoading}
              />
            ) : (
              <div
                style={{
                  height: '100%',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#aaa',
                  fontSize: 12,
                }}
              >
                点击文件树中的文件以打开
              </div>
            )}
          </div>
        </div>
      </main>

      {/* LaTeX 放大 Modal */}
      {latexModal && (
        <LatexModal latex={latexModal} onClose={() => setLatexModal(null)} />
      )}
    </div>
  );
}
