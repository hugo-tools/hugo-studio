import { useCallback, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  ClipboardCopy,
  FileCode,
  FileText,
  File as FileIcon,
  ImageIcon,
  Palette,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import {
  describeError,
  tauri,
  type AssetContext,
  type AssetRef,
  type Site,
} from "@/lib/tauri";

export type MediaScope = "static" | "assets" | "bundle";

interface Props {
  site: Site;
  /** When set, the "Bundle" scope is available and points at this content
   *  path (the index file of a Branch / Leaf bundle). */
  bundleContentId?: string | null;
  bundleLabel?: string | null;
  /** Default scope to land on. The picker passes `"static"` because it
   *  expects URL-style links; the standalone library starts on whatever
   *  the user looked at last (defaults to `static`). */
  initialScope?: MediaScope;
  /** When set, each media card gets a primary "Insert" action that
   *  invokes this callback. Used by the editor's media picker. */
  onSelect?: (asset: AssetRef) => void;
}

export function MediaLibrary({
  site,
  bundleContentId,
  bundleLabel,
  initialScope,
  onSelect,
}: Props) {
  const queryClient = useQueryClient();

  const bundleAvailable = !!bundleContentId;
  const [scope, setScope] = useState<MediaScope>(
    initialScope ?? (bundleAvailable ? "bundle" : "static"),
  );
  const [filter, setFilter] = useState("");
  const [staticSubpath, setStaticSubpath] = useState("img");

  const queryKey = useMemo(
    () => ["media", site.id, scope, scope === "bundle" ? bundleContentId : null],
    [site.id, scope, bundleContentId],
  );

  const list = useQuery<AssetRef[]>({
    queryKey,
    queryFn: () => {
      switch (scope) {
        case "static":
          return tauri.assetListStatic(site.id);
        case "assets":
          return tauri.assetListAssets(site.id);
        case "bundle":
          return tauri.assetList(site.id, bundleContentId ?? null);
      }
    },
    enabled: scope !== "bundle" || !!bundleContentId,
  });

  const refresh = () => queryClient.invalidateQueries({ queryKey });

  const importAt = useCallback(
    async (sources: string[]) => {
      const ctx = scopeToContext(scope, bundleContentId, staticSubpath);
      if (!ctx) return [];
      const out: AssetRef[] = [];
      for (const source of sources) {
        out.push(await tauri.assetImport(site.id, source, ctx));
      }
      return out;
    },
    [site.id, scope, bundleContentId, staticSubpath],
  );

  const add = useMutation({
    mutationFn: async () => {
      const picked = await openFile({
        multiple: true,
        directory: false,
      });
      if (!picked) return [];
      const sources = Array.isArray(picked) ? picked : [picked];
      return importAt(sources);
    },
    onSuccess: (refs) => {
      if (refs.length) refresh();
    },
    onError: (e) => alert(describeError(e)),
  });

  const remove = useMutation({
    mutationFn: (asset: AssetRef) => tauri.assetDelete(site.id, asset.id),
    onSuccess: refresh,
    onError: (e) => alert(describeError(e)),
  });

  const filtered = useMemo(() => {
    const all = list.data ?? [];
    const f = filter.trim().toLowerCase();
    if (!f) return all;
    return all.filter(
      (a) =>
        a.name.toLowerCase().includes(f) ||
        a.contextLabel.toLowerCase().includes(f),
    );
  }, [list.data, filter]);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex flex-col gap-2 border-b px-4 py-3">
        <div className="flex items-center justify-between gap-2">
          <Tabs
            value={scope}
            onValueChange={(v) => setScope(v as MediaScope)}
            className=""
          >
            <TabsList>
              <TabsTrigger value="static">Static</TabsTrigger>
              <TabsTrigger value="assets">Assets</TabsTrigger>
              <TabsTrigger value="bundle" disabled={!bundleAvailable}>
                Bundle{bundleLabel ? ` — ${bundleLabel}` : ""}
              </TabsTrigger>
            </TabsList>
          </Tabs>
          <div className="flex items-center gap-2">
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={refresh}
              disabled={list.isFetching}
            >
              <RefreshCw
                className={cn("size-3.5", list.isFetching && "animate-spin")}
              />
            </Button>
            <Button
              type="button"
              size="sm"
              onClick={() => add.mutate()}
              disabled={add.isPending}
            >
              <Plus className="size-3.5" />
              Add files
            </Button>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Input
            type="search"
            placeholder="Filter by name…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="h-8 max-w-xs text-xs"
          />
          {scope === "static" && (
            <div className="flex items-center gap-1 font-mono text-[11px] text-muted-foreground">
              <span>add to static/</span>
              <Input
                type="text"
                value={staticSubpath}
                onChange={(e) => setStaticSubpath(e.target.value)}
                placeholder="(empty for root)"
                className="h-7 w-32 px-2 py-0 text-[11px]"
              />
            </div>
          )}
          <span className="ml-auto text-[11px] text-muted-foreground">
            {list.data ? `${filtered.length} of ${list.data.length}` : ""}
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        {list.isPending && (
          <p className="text-xs text-muted-foreground">Loading…</p>
        )}
        {list.isError && (
          <p className="text-xs text-destructive">
            {describeError(list.error)}
          </p>
        )}
        {list.data && filtered.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {emptyHint(scope, !!list.data?.length)}
          </p>
        )}

        <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3">
          {filtered.map((a) => (
            <MediaCard
              key={a.id}
              asset={a}
              onInsert={onSelect ? () => onSelect(a) : undefined}
              onCopy={() => copyToClipboard(a.relativeLink)}
              onDelete={() => {
                if (confirm(`Delete ${a.name}?`)) remove.mutate(a);
              }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function MediaCard({
  asset,
  onInsert,
  onCopy,
  onDelete,
}: {
  asset: AssetRef;
  onInsert?: () => void;
  onCopy: () => void;
  onDelete: () => void;
}) {
  const isImage = asset.kind === "image";
  const Icon = iconFor(asset.kind);

  return (
    <div
      className={cn(
        "group flex flex-col overflow-hidden rounded-md border bg-background",
        onInsert && "cursor-pointer transition-colors hover:border-primary",
      )}
      onClick={onInsert}
      onKeyDown={(e) => {
        if (onInsert && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          onInsert();
        }
      }}
      role={onInsert ? "button" : undefined}
      tabIndex={onInsert ? 0 : undefined}
      title={asset.relativeLink}
    >
      <div className="relative flex aspect-square items-center justify-center overflow-hidden bg-muted/40">
        {isImage ? (
          <img
            src={convertFileSrc(asset.path)}
            alt={asset.name}
            className="h-full w-full object-contain"
            loading="lazy"
          />
        ) : (
          <Icon className="size-10 text-muted-foreground" />
        )}
      </div>
      <div className="flex flex-col gap-0.5 px-2 py-1.5">
        <span className="truncate text-xs font-medium" title={asset.name}>
          {asset.name}
        </span>
        <span className="truncate text-[10px] text-muted-foreground">
          {asset.contextLabel} · {formatSize(asset.size)}
        </span>
      </div>
      <div className="flex items-center justify-between border-t bg-muted/20 px-1 py-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        <Button
          type="button"
          size="icon"
          variant="ghost"
          className="size-7"
          onClick={(e) => {
            e.stopPropagation();
            onCopy();
          }}
          aria-label="Copy link"
          title={`Copy ${asset.relativeLink}`}
        >
          <ClipboardCopy className="size-3.5" />
        </Button>
        <Button
          type="button"
          size="icon"
          variant="ghost"
          className="size-7 text-destructive hover:text-destructive"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          aria-label={`Delete ${asset.name}`}
        >
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}

function scopeToContext(
  scope: MediaScope,
  bundleContentId: string | null | undefined,
  staticSubpath: string,
): AssetContext | null {
  switch (scope) {
    case "bundle":
      if (!bundleContentId) return null;
      return { kind: "bundle", contentId: bundleContentId };
    case "static":
      return { kind: "static", subpath: staticSubpath.trim() };
    case "assets":
      return { kind: "assets", subpath: "" };
  }
}

function emptyHint(scope: MediaScope, hasAny: boolean): string {
  if (hasAny) return "No matches for that filter.";
  switch (scope) {
    case "static":
      return "static/ is empty. Add a file to make it reachable at /<file>.";
    case "assets":
      return "assets/ is empty. Files here go through Hugo Pipes (resources.Get).";
    case "bundle":
      return "No sibling files yet. Add an image to use it as ![](file.jpg) in the body.";
  }
}

function iconFor(kind: AssetRef["kind"]) {
  switch (kind) {
    case "image":
      return ImageIcon;
    case "script":
      return FileCode;
    case "style":
      return Palette;
    case "document":
      return FileText;
    default:
      return FileIcon;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function copyToClipboard(text: string) {
  if (navigator.clipboard?.writeText) {
    void navigator.clipboard.writeText(text);
  } else {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    try {
      document.execCommand("copy");
    } finally {
      document.body.removeChild(ta);
    }
  }
}
