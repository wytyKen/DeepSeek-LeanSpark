// DeepSeek-LeanSpark 前端入口。
// 布局：顶部栏 + 主区（左：聊天面板，右：右侧栏三 Tab）。
import { useEffect, useState } from 'react';
import { useAgent, lastLeanCodeFromMessages } from './hooks/useAgent';
import { useWorkspace } from './hooks/useWorkspace';
import { useProofGraph } from './hooks/useProofGraph';
import { useSettings } from './hooks/useSettings';
import { useLeanInstall } from './hooks/useLeanInstall';
import { ChatPanel } from './components/chat/ChatPanel';
import { RightSidebar } from './components/sidebar/RightSidebar';
import { WorkspaceSwitcher } from './components/workspace/WorkspaceSwitcher';
import { CodeEditor } from './components/workspace/CodeEditor';
import { LatexModal } from './components/common/LatexModal';
import { SettingsModal } from './components/common/SettingsModal';
import { LeanInstallModal } from './components/common/LeanInstallModal';
import 'katex/dist/katex.min.css';
import './styles/chat.css';

export default function App() {
  const [thinking, setThinking] = useState(false);
  const { messages, send, reset, isRunning } = useAgent({ thinking });
  const workspace = useWorkspace();
  const proofGraph = useProofGraph();
  // API Key 设置：启动时自动拉取配置状态；未配置时 forceOpen=true 强制弹 Modal
  const settings = useSettings();
  // Lean4 安装检测：启动时拉取；未安装时弹引导 Modal（用户可关闭，不强制阻塞）
  const leanInstall = useLeanInstall();

  // 当前在编辑器中打开的文件
  const [openFile, setOpenFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState('');
  const [fileDirty, setFileDirty] = useState(false);
  const [fileLoading, setFileLoading] = useState(false);

  // 当前要放大的 LaTeX 公式
  const [latexModal, setLatexModal] = useState<string | null>(null);

  // 设置 Modal 显隐：
  // - settingsOpen=false 表示用户主动关闭过 Modal（仅 forceOpen=false 时生效）
  // - 当 settings.configured=false 时，无论 settingsOpen 如何都强制显示（forceOpen=true）
  const [settingsOpen, setSettingsOpen] = useState(false);
  const showSettingsModal = settingsOpen || !settings.configured;
  const forceSettingsOpen = !settings.configured;

  // Lean 安装引导 Modal 显隐：
  // - 启动时若 leanInstall.installed=false 自动弹出
  // - 用户关闭后不再自动弹出，但可通过 header 按钮重新打开
  const [leanInstallOpen, setLeanInstallOpen] = useState(false);
  useEffect(() => {
    if (!leanInstall.loading && !leanInstall.installed) {
      setLeanInstallOpen(true);
    }
  }, [leanInstall.loading, leanInstall.installed]);

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
        {/* Lean4 状态按钮：未安装时显示警告色，点击重新打开引导 Modal */}
        <button
          type="button"
          onClick={() => setLeanInstallOpen(true)}
          className="header-lean-status-btn"
          aria-label="Lean4 安装状态"
          style={{
            fontSize: 12,
            padding: '4px 10px',
            background: leanInstall.installed ? '#f0fdf4' : '#fef3c7',
            color: leanInstall.installed ? '#15803d' : '#b45309',
            border: `1px solid ${leanInstall.installed ? '#bbf7d0' : '#fde68a'}`,
            borderRadius: 4,
            cursor: 'pointer',
          }}
        >
          {leanInstall.loading
            ? 'Lean 检测中...'
            : leanInstall.installed
              ? `Lean ✓ ${leanInstall.version ?? ''}`.trim()
              : 'Lean 未安装'}
        </button>
        {/* 设置按钮：显示当前 API Key 配置状态，点击打开 Modal */}
        <button
          type="button"
          onClick={() => setSettingsOpen(true)}
          className="header-settings-btn"
          aria-label="API Key 设置"
          style={{
            fontSize: 12,
            padding: '4px 10px',
            background: settings.configured ? '#f0fdf4' : '#fef3c7',
            color: settings.configured ? '#15803d' : '#b45309',
            border: `1px solid ${settings.configured ? '#bbf7d0' : '#fde68a'}`,
            borderRadius: 4,
            cursor: 'pointer',
          }}
        >
          {settings.configured ? `已配置 · ${settings.model}` : '未配置 API Key'}
        </button>
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

      {/* API Key 设置 Modal */}
      <SettingsModal
        open={showSettingsModal}
        currentModel={settings.model}
        forceOpen={forceSettingsOpen}
        onSubmit={settings.setApiKey}
        onClose={() => setSettingsOpen(false)}
      />

      {/* Lean4 安装引导 Modal */}
      <LeanInstallModal
        open={leanInstallOpen}
        status={leanInstall}
        onClose={() => setLeanInstallOpen(false)}
      />
    </div>
  );
}
