import type { ContentSummary } from "@/lib/tauri";

export interface TreeNode {
  item: ContentSummary;
  children: TreeNode[];
}

/**
 * Group a flat content list into a hierarchy by `id` path. The flat list
 * already contains every directory + page at the right depth; we just need
 * to wire the parent/child links by walking the path components.
 */
export function buildTree(items: ContentSummary[]): TreeNode[] {
  const sorted = [...items].sort((a, b) => a.id.localeCompare(b.id));
  const byId = new Map<string, TreeNode>();
  const roots: TreeNode[] = [];

  for (const item of sorted) {
    const node: TreeNode = { item, children: [] };
    byId.set(item.id, node);
    const slashIdx = item.id.lastIndexOf("/");
    if (slashIdx === -1) {
      roots.push(node);
      continue;
    }
    const parentId = item.id.slice(0, slashIdx);
    const parent = byId.get(parentId);
    if (parent) parent.children.push(node);
    else roots.push(node);
  }
  sortNodes(roots);
  return roots;
}

function sortNodes(nodes: TreeNode[]) {
  // Folders (Section / BranchBundle / LeafBundle) first, then pages — both
  // alphabetical inside their group.
  nodes.sort((a, b) => {
    const aFolder = a.item.kind !== "singlePage";
    const bFolder = b.item.kind !== "singlePage";
    if (aFolder !== bFolder) return aFolder ? -1 : 1;
    return (a.item.title ?? a.item.id).localeCompare(b.item.title ?? b.item.id);
  });
  for (const n of nodes) sortNodes(n.children);
}
