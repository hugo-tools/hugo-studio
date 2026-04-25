import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  ChevronRight,
  File,
  FileText,
  Folder,
  FolderOpen,
  Layers,
  Package,
} from "lucide-react";

import {
  describeError,
  tauri,
  type ContentScanResult,
  type ContentSummary,
  type Site,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { useWorkspaceStore } from "@/store/workspace";
import { Button } from "@/components/ui/button";
import { ArrowUpDown, FilePlus2 } from "lucide-react";
import { LanguageSwitcher } from "./LanguageSwitcher";
import { NewContentDialog } from "./NewContentDialog";
import {
  buildTree,
  SORT_LABELS,
  type SortMode,
  type TreeNode,
} from "./buildTree";

interface Props {
  site: Site;
}

export function ContentTree({ site }: Props) {
  const queryClient = useQueryClient();
  const [activeLang, setActiveLang] = useState<string | null>(null);
  const [newOpen, setNewOpen] = useState(false);
  const [sortBy, setSortBy] = useState<SortMode>("name");

  const scan = useQuery<ContentScanResult>({
    queryKey: ["content", site.id],
    queryFn: () => tauri.contentList(site.id),
  });

  // Subscribe to backend file-watcher pings — invalidate the content query
  // (and the config one, since the user might be editing hugo.toml too).
  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | null = null;
    listen<{ paths: string[] }>("site:changed", () => {
      if (cancelled) return;
      queryClient.invalidateQueries({ queryKey: ["content", site.id] });
      queryClient.invalidateQueries({ queryKey: ["config", site.id] });
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [queryClient, site.id]);

  // Initialise the active language from the backend default once the scan
  // arrives. The user can override later.
  useEffect(() => {
    if (!scan.data || activeLang) return;
    setActiveLang(scan.data.languageInfo.defaultLanguage);
  }, [scan.data, activeLang]);

  const filteredItems = useMemo<ContentSummary[]>(() => {
    if (!scan.data) return [];
    if (scan.data.languageInfo.strategy === "mono" || !activeLang) {
      return scan.data.items;
    }
    return scan.data.items.filter((i) => i.language === activeLang);
  }, [scan.data, activeLang]);

  const translationsById = useMemo(() => {
    const map = new Map<string, Set<string>>();
    if (!scan.data) return map;
    for (const item of scan.data.items) {
      const set = map.get(item.id) ?? new Set();
      set.add(item.language);
      map.set(item.id, set);
    }
    return map;
  }, [scan.data]);

  const tree = useMemo(
    () => buildTree(filteredItems, sortBy),
    [filteredItems, sortBy],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-col gap-1 border-b px-3 py-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
              Content
            </span>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="size-6"
              onClick={() => setNewOpen(true)}
              aria-label="New content"
              title="New content"
            >
              <FilePlus2 className="size-3.5" />
            </Button>
          </div>
          {scan.data && (
            <LanguageSwitcher
              languages={scan.data.languageInfo.languages}
              active={activeLang ?? scan.data.languageInfo.defaultLanguage}
              onChange={setActiveLang}
            />
          )}
        </div>
        <label
          className="flex items-center gap-1.5 text-[10px] text-muted-foreground"
          title="Tree sort order"
        >
          <ArrowUpDown className="size-3" />
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortMode)}
            className="flex-1 rounded border bg-background px-1 py-0.5 text-[10px]"
          >
            {(Object.keys(SORT_LABELS) as SortMode[]).map((mode) => (
              <option key={mode} value={mode}>
                {SORT_LABELS[mode]}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="flex-1 overflow-auto px-1 py-2">
        {scan.isPending && (
          <p className="px-3 text-xs text-muted-foreground">Scanning…</p>
        )}
        {scan.isError && (
          <p className="px-3 text-xs text-destructive">
            {describeError(scan.error)}
          </p>
        )}
        {scan.data && tree.length === 0 && (
          <p className="px-3 text-xs text-muted-foreground">
            No content yet under <code className="font-mono">content/</code>.
          </p>
        )}
        <ul className="space-y-0.5">
          {tree.map((node) => (
            <TreeRow
              key={node.item.id + node.item.language}
              node={node}
              translations={translationsById}
              activeLang={
                activeLang ?? scan.data?.languageInfo.defaultLanguage ?? ""
              }
              allLangs={
                scan.data?.languageInfo.languages.map((l) => l.code) ?? []
              }
            />
          ))}
        </ul>
      </div>

      <NewContentDialog
        open={newOpen}
        onOpenChange={setNewOpen}
        site={site}
        scan={scan.data ?? null}
      />
    </div>
  );
}

function TreeRow({
  node,
  translations,
  activeLang,
  allLangs,
  depth = 0,
}: {
  node: TreeNode;
  translations: Map<string, Set<string>>;
  activeLang: string;
  allLangs: string[];
  depth?: number;
}) {
  const [open, setOpen] = useState(depth < 1);
  const selectContent = useWorkspaceStore((s) => s.selectContent);
  const selectedPath = useWorkspaceStore((s) => s.selection?.path);
  const item = node.item;
  const isFolder = item.kind !== "singlePage";
  const isEditableBundle =
    item.kind === "branchBundle" || item.kind === "leafBundle";
  const isEditable = item.kind === "singlePage" || isEditableBundle;
  const Icon = iconFor(item.kind, open);
  const isSelected = selectedPath === item.path;

  function handleClick() {
    if (isEditable) {
      selectContent({
        path: item.path,
        id: item.id,
        language: item.language,
        title: item.title,
      });
    }
    if (isFolder) setOpen((v) => !v);
  }

  const otherLangs =
    allLangs.length > 1
      ? allLangs.filter(
          (c) => c !== activeLang && translations.get(item.id)?.has(c),
        )
      : [];
  const missing =
    allLangs.length > 1
      ? allLangs.filter(
          (c) => c !== activeLang && !translations.get(item.id)?.has(c),
        )
      : [];

  return (
    <li>
      <button
        type="button"
        onClick={handleClick}
        className={cn(
          "group flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-sm transition-colors hover:bg-muted",
          item.draft && "italic text-muted-foreground",
          isSelected && "bg-accent text-accent-foreground hover:bg-accent",
        )}
        style={{ paddingLeft: `${depth * 12 + 6}px` }}
      >
        {isFolder ? (
          <ChevronRight
            className={cn(
              "size-3.5 shrink-0 text-muted-foreground transition-transform",
              open && "rotate-90",
            )}
          />
        ) : (
          <span className="w-3.5" />
        )}
        <Icon className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate">{item.title ?? item.id}</span>
        {item.draft && (
          <span className="ml-1 rounded bg-amber-100 px-1 text-[10px] font-medium text-amber-900">
            draft
          </span>
        )}
        <span className="ml-auto flex items-center gap-1 text-[10px] text-muted-foreground">
          {otherLangs.length > 0 && (
            <span title={`Translations: ${otherLangs.join(", ")}`}>
              + {otherLangs.join("/")}
            </span>
          )}
          {missing.length > 0 && (
            <span
              className="text-amber-600"
              title={`Missing: ${missing.join(", ")}`}
            >
              ⚠ {missing.join("/")}
            </span>
          )}
        </span>
      </button>
      {isFolder && open && node.children.length > 0 && (
        <ul className="space-y-0.5">
          {node.children.map((child) => (
            <TreeRow
              key={child.item.id + child.item.language}
              node={child}
              translations={translations}
              activeLang={activeLang}
              allLangs={allLangs}
              depth={depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

function iconFor(kind: ContentSummary["kind"], open: boolean) {
  switch (kind) {
    case "branchBundle":
      return Layers;
    case "leafBundle":
      return Package;
    case "section":
      return open ? FolderOpen : Folder;
    case "singlePage":
      return FileText;
    default:
      return File;
  }
}
