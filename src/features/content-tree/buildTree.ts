import type { ContentSummary } from "@/lib/tauri";

export interface TreeNode {
  item: ContentSummary;
  children: TreeNode[];
}

export type SortMode = "name" | "newest" | "oldest";

export const SORT_LABELS: Record<SortMode, string> = {
  name: "Name (A→Z)",
  newest: "Newest first",
  oldest: "Oldest first",
};

/**
 * Group a flat content list into a hierarchy by `id` path. The flat list
 * already contains every directory + page at the right depth; we just need
 * to wire the parent/child links by walking the path components.
 *
 * `sortBy` controls the sibling order at every level:
 * - `name`    — alphabetical by title (existing default)
 * - `newest`  — front-matter date desc, no-date items pushed to the end
 * - `oldest`  — front-matter date asc, no-date items pushed to the end
 *
 * Folders always come before files within their group, regardless of sort.
 */
export function buildTree(
  items: ContentSummary[],
  sortBy: SortMode = "name",
): TreeNode[] {
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
  sortNodes(roots, sortBy);
  return roots;
}

function sortNodes(nodes: TreeNode[], sortBy: SortMode) {
  nodes.sort((a, b) => compare(a, b, sortBy));
  for (const n of nodes) sortNodes(n.children, sortBy);
}

function compare(a: TreeNode, b: TreeNode, sortBy: SortMode): number {
  // Folders always before pages, irrespective of sort mode.
  const aFolder = a.item.kind !== "singlePage";
  const bFolder = b.item.kind !== "singlePage";
  if (aFolder !== bFolder) return aFolder ? -1 : 1;

  if (sortBy !== "name") {
    const aDate = parseDate(a.item.date);
    const bDate = parseDate(b.item.date);
    // Items missing a date fall to the bottom under both date modes —
    // they're typically section index files where date is meaningless.
    if (aDate == null && bDate == null) {
      return nameOrder(a, b);
    }
    if (aDate == null) return 1;
    if (bDate == null) return -1;
    return sortBy === "newest" ? bDate - aDate : aDate - bDate;
  }
  return nameOrder(a, b);
}

function nameOrder(a: TreeNode, b: TreeNode): number {
  return (a.item.title ?? a.item.id).localeCompare(b.item.title ?? b.item.id);
}

function parseDate(raw: string | null | undefined): number | null {
  if (!raw) return null;
  const t = Date.parse(raw);
  return Number.isNaN(t) ? null : t;
}
