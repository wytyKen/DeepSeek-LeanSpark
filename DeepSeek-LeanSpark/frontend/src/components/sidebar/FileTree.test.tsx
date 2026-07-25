// FileTree 组件测试
// 验证：空节点提示、目录展开折叠、文件点击回调、选中高亮、文件图标。
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { FileTree } from './FileTree';
import type { FileNode } from '../../types';

const sampleTree: FileNode = {
  name: 'workspace',
  path: '',
  kind: 'dir',
  children: [
    { name: 'a.lean', path: 'a.lean', kind: 'file', size: 100 },
    { name: 'b.md', path: 'b.md', kind: 'file', size: 50 },
    {
      name: 'sub',
      path: 'sub',
      kind: 'dir',
      children: [
        { name: 'c.lean', path: 'sub/c.lean', kind: 'file', size: 30 },
        { name: '.hidden', path: 'sub/.hidden', kind: 'file', size: 1 },
      ],
    },
  ],
};

describe('FileTree', () => {
  it('shows placeholder when node is null', () => {
    render(<FileTree node={null} onOpenFile={() => {}} />);
    expect(screen.getByText('未打开工作区')).toBeInTheDocument();
  });

  it('renders all top-level files and folders', () => {
    render(<FileTree node={sampleTree} onOpenFile={() => {}} />);
    expect(screen.getByText('a.lean')).toBeInTheDocument();
    expect(screen.getByText('b.md')).toBeInTheDocument();
    expect(screen.getByText('sub')).toBeInTheDocument();
  });

  it('renders nested files in expanded folder by default', () => {
    render(<FileTree node={sampleTree} onOpenFile={() => {}} />);
    expect(screen.getByText('c.lean')).toBeInTheDocument();
  });

  it('collapses folder on click and removes nested files', () => {
    render(<FileTree node={sampleTree} onOpenFile={() => {}} />);
    fireEvent.click(screen.getByText('sub'));
    expect(screen.queryByText('c.lean')).toBeNull();
  });

  it('expands folder again on second click', () => {
    render(<FileTree node={sampleTree} onOpenFile={() => {}} />);
    const folder = screen.getByText('sub');
    fireEvent.click(folder); // 折叠
    expect(screen.queryByText('c.lean')).toBeNull();
    fireEvent.click(folder); // 展开
    expect(screen.getByText('c.lean')).toBeInTheDocument();
  });

  it('calls onOpenFile with relative path when file clicked', () => {
    const handler = vi.fn();
    render(<FileTree node={sampleTree} onOpenFile={handler} />);
    fireEvent.click(screen.getByText('a.lean'));
    expect(handler).toHaveBeenCalledWith('a.lean');
  });

  it('does not call onOpenFile when folder clicked', () => {
    const handler = vi.fn();
    render(<FileTree node={sampleTree} onOpenFile={handler} />);
    fireEvent.click(screen.getByText('sub'));
    expect(handler).not.toHaveBeenCalled();
  });

  it('highlights selected file with different color', () => {
    const { container } = render(
      <FileTree node={sampleTree} selectedPath="a.lean" onOpenFile={() => {}} />,
    );
    const selectedItem = container.querySelector('.tree-item.file.selected') as HTMLElement;
    expect(selectedItem).toBeTruthy();
    // jsdom 将 #2563eb / #eff6ff 规范化为 rgb 形式
    expect(selectedItem.style.color).toBe('rgb(37, 99, 235)');
    expect(selectedItem.style.background).toBe('rgb(239, 246, 255)');
  });

  it('does not highlight non-selected files', () => {
    const { container } = render(
      <FileTree node={sampleTree} selectedPath="a.lean" onOpenFile={() => {}} />,
    );
    const bItem = screen.getByText('b.md').parentElement as HTMLElement;
    expect(bItem.className).not.toContain('selected');
  });

  it('renders different icons for different file types', () => {
    render(<FileTree node={sampleTree} onOpenFile={() => {}} />);
    // .lean → 📘 (a.lean + c.lean 共 2 个), .md → 📝 (1 个)
    expect(screen.getAllByText('📘').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('📝')).toBeInTheDocument();
  });

  it('handles empty folder (no children)', () => {
    const emptyTree: FileNode = {
      name: 'root',
      path: '',
      kind: 'dir',
      children: [],
    };
    render(<FileTree node={emptyTree} onOpenFile={() => {}} />);
    // 不应崩溃，无子项
    expect(screen.queryByText(/.lean|.md/)).toBeNull();
  });

  it('handles root being a file (edge case)', () => {
    const fileRoot: FileNode = {
      name: 'root',
      path: '',
      kind: 'file',
    };
    render(<FileTree node={fileRoot} onOpenFile={() => {}} />);
    // 根节点为文件时，TreeItem 返回 null
    expect(screen.queryByText('root')).toBeNull();
  });
});
