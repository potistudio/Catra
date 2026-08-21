import type { PlaylistNode } from "$lib/types";

/** 木を1列に潰した表示用の行。畳まれた枝の中は行にならない。 */
export interface PlaylistTreeRow {
  node: PlaylistNode;
  depth: number;
  hasChildren: boolean;
  expanded: boolean;
}

function groupByParent(nodes: PlaylistNode[]): Map<number | null, PlaylistNode[]> {
  const byParent = new Map<number | null, PlaylistNode[]>();
  for (const node of nodes) {
    const siblings = byParent.get(node.parentId);
    if (siblings) siblings.push(node);
    else byParent.set(node.parentId, [node]);
  }
  for (const siblings of byParent.values()) {
    siblings.sort((a, b) => a.position - b.position || a.id - b.id);
  }
  return byParent;
}

/** 先行順に歩いて、字下げ付きの行にする。 */
export function playlistTreeRows(
  nodes: PlaylistNode[],
  expandedIds: Iterable<number>,
): PlaylistTreeRow[] {
  const byParent = groupByParent(nodes);
  const expanded = new Set(expandedIds);
  const rows: PlaylistTreeRow[] = [];

  function walk(parentId: number | null, depth: number) {
    const children = byParent.get(parentId);
    if (!children) return;
    for (const node of children) {
      const hasChildren = (byParent.get(node.id)?.length ?? 0) > 0;
      const open = hasChildren && expanded.has(node.id);
      rows.push({ node, depth, hasChildren, expanded: open });
      if (open) walk(node.id, depth + 1);
    }
  }

  walk(null, 0);
  return rows;
}

/** 自分と、自分より下にぶら下がっている全部の ID。移動先の禁止判定に使う。 */
export function subtreeIds(nodes: PlaylistNode[], rootId: number): Set<number> {
  const byParent = groupByParent(nodes);
  const ids = new Set<number>([rootId]);

  function walk(parentId: number) {
    for (const child of byParent.get(parentId) ?? []) {
      ids.add(child.id);
      walk(child.id);
    }
  }

  walk(rootId);
  return ids;
}

/** 根から自分までの ID。選択中のプレイリストを畳まれた枝から掘り出すのに使う。 */
export function ancestorIds(nodes: PlaylistNode[], id: number): number[] {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const path: number[] = [];
  let current = byId.get(id)?.parentId ?? null;
  while (current != null) {
    path.unshift(current);
    current = byId.get(current)?.parentId ?? null;
  }
  return path;
}

/** フォルダにだけ子を入れられる。プレイリストの中にプレイリストは作れない。 */
export function canDropInto(nodes: PlaylistNode[], dragId: number, targetId: number | null): boolean {
  if (targetId == null) return true;
  if (dragId === targetId) return false;
  const target = nodes.find((node) => node.id === targetId);
  if (!target || target.kind !== "folder") return false;
  return !subtreeIds(nodes, dragId).has(targetId);
}
