// 证明依赖图视图：用 React Flow 渲染有向图，dagre 自动布局。
// 节点：theorem（蓝色）、lemma（绿色）、外部引理（灰色虚线边框）。
import { useMemo } from 'react';
import ReactFlow, { Background, Controls, MiniMap, MarkerType } from 'reactflow';
import dagre from 'dagre';
import type { GraphEdge, GraphNode, ProofGraph } from '../../types';
import 'reactflow/dist/style.css';

interface Props {
  graph: ProofGraph | null;
}

const NODE_WIDTH = 160;
const NODE_HEIGHT = 36;

export function ProofGraphView({ graph }: Props) {
  const { nodes, edges } = useMemo(() => {
    if (!graph) return { nodes: [], edges: [] };
    return layoutGraph(graph.nodes, graph.edges);
  }, [graph]);

  if (!graph || graph.nodes.length === 0) {
    return (
      <div
        style={{
          padding: 16,
          color: '#888',
          fontSize: 13,
          textAlign: 'center',
        }}
      >
        尚无证明依赖图
        <div style={{ fontSize: 11, color: '#aaa', marginTop: 4 }}>
          提交 Lean4 代码后自动生成
        </div>
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        attributionPosition="bottom-right"
        nodesDraggable
        nodesConnectable={false}
        zoomOnScroll
        panOnScroll
      >
        <Background color="#aaa" gap={16} />
        <Controls showInteractive={false} />
        <MiniMap
          nodeStrokeColor="#333"
          nodeColor={(n) => (n.data?.color as string) ?? '#ddd'}
        />
      </ReactFlow>
      {graph.note && (
        <div
          style={{
            position: 'absolute',
            bottom: 4,
            left: 4,
            right: 4,
            fontSize: 10,
            color: '#999',
            background: 'rgba(255,255,255,0.85)',
            padding: '2px 6px',
            borderRadius: 3,
            pointerEvents: 'none',
          }}
        >
          {graph.note}
        </div>
      )}
    </div>
  );
}

// 用 dagre 计算节点位置
function layoutGraph(nodes: GraphNode[], edges: GraphEdge[]) {
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: 'TB', nodesep: 40, ranksep: 60 });
  g.setDefaultEdgeLabel(() => ({}));

  for (const n of nodes) {
    g.setNode(n.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }
  for (const e of edges) {
    if (g.hasNode(e.from) && g.hasNode(e.to)) {
      g.setEdge(e.from, e.to);
    }
  }
  dagre.layout(g);

  const colorFor = (n: GraphNode): string => {
    if (n.external) return '#9ca3af';
    if (n.kind === 'theorem') return '#2563eb';
    if (n.kind === 'lemma') return '#059669';
    return '#6b7280';
  };

  const rfNodes = nodes.map((n) => {
    const pos = g.node(n.id);
    const color = colorFor(n);
    return {
      id: n.id,
      position: { x: (pos?.x ?? 0) - NODE_WIDTH / 2, y: (pos?.y ?? 0) - NODE_HEIGHT / 2 },
      data: {
        label: n.name,
        kind: n.kind,
        external: n.external,
        color,
      },
      style: {
        background: n.external ? '#f9fafb' : '#fff',
        color,
        border: `1px ${n.external ? 'dashed' : 'solid'} ${color}`,
        borderRadius: 6,
        fontSize: 12,
        padding: '4px 8px',
        width: NODE_WIDTH,
      },
    };
  });

  const rfEdges = edges.map((e, i) => ({
    id: `e-${i}-${e.from}-${e.to}`,
    source: e.from,
    target: e.to,
    type: 'smoothstep',
    style: { stroke: '#bbb', strokeWidth: 1 },
    markerEnd: {
      type: MarkerType.ArrowClosed,
      width: 16,
      height: 16,
      color: '#bbb',
    },
  }));

  return { nodes: rfNodes, edges: rfEdges };
}
