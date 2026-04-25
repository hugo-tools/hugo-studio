import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  FileText,
  ImageIcon,
  FileCode,
  Palette,
  File,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { describeError, tauri, type AssetRef, type SiteId } from "@/lib/tauri";

interface Props {
  siteId: SiteId;
  contentId: string | null;
  /** Called when the user clicks an asset row — typically inserts the
   *  markdown link at the editor's caret. */
  onInsertLink: (asset: AssetRef) => void;
}

export function BundleAssetsPanel({ siteId, contentId, onInsertLink }: Props) {
  const queryClient = useQueryClient();
  const list = useQuery<AssetRef[]>({
    queryKey: ["assets", siteId, contentId],
    queryFn: () => tauri.assetList(siteId, contentId),
    enabled: !!contentId,
  });

  const remove = useMutation({
    mutationFn: (asset: AssetRef) => tauri.assetDelete(siteId, asset.id),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: ["assets", siteId, contentId],
      }),
    onError: (e) => alert(describeError(e)),
  });

  if (!contentId) return null;

  return (
    <aside className="flex h-full flex-col border-l bg-muted/20">
      <header className="flex items-center justify-between border-b px-3 py-2">
        <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Bundle assets
        </span>
        <span className="text-[10px] text-muted-foreground">
          {list.data?.length ?? 0}
        </span>
      </header>
      <div className="flex-1 overflow-auto px-1 py-2">
        {list.isPending && (
          <p className="px-2 text-xs text-muted-foreground">Loading…</p>
        )}
        {list.isError && (
          <p className="px-2 text-xs text-destructive">
            {describeError(list.error)}
          </p>
        )}
        {list.data && list.data.length === 0 && (
          <p className="px-2 text-xs text-muted-foreground">
            Drop a file on the editor to add an asset.
          </p>
        )}
        <ul className="space-y-0.5">
          {list.data?.map((asset) => {
            const Icon = iconFor(asset.kind);
            return (
              <li key={asset.id} className="group flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => onInsertLink(asset)}
                  title={`Insert ![](${asset.relativeLink})`}
                  className="flex flex-1 items-center gap-2 truncate rounded px-2 py-1 text-left text-xs hover:bg-accent"
                >
                  <Icon className="size-3.5 shrink-0 text-muted-foreground" />
                  <span className="flex-1 truncate font-mono">
                    {asset.name}
                  </span>
                  <span className="shrink-0 text-[10px] text-muted-foreground">
                    {formatSize(asset.size)}
                  </span>
                </button>
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-6 opacity-0 group-hover:opacity-100"
                  onClick={() => {
                    if (confirm(`Delete ${asset.name}?`)) remove.mutate(asset);
                  }}
                  aria-label={`Delete ${asset.name}`}
                >
                  <Trash2 className="size-3" />
                </Button>
              </li>
            );
          })}
        </ul>
      </div>
    </aside>
  );
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
      return File;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
