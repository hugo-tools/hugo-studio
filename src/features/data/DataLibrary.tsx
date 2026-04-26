import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  FileJson,
  FileSpreadsheet,
  FileText,
  Plus,
  Save,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { rectMatcher, registerDropRegion } from "@/lib/dnd/regions";
import {
  describeError,
  tauri,
  type DataFile,
  type DataFileContent,
  type Site,
} from "@/lib/tauri";
import { BodyEditor } from "@/features/editor/BodyEditor";
import { CsvGrid } from "./CsvGrid";

/** Extensions accepted by the OS drop importer. Mirrors the backend's
 *  `IMPORT_EXTENSIONS`. Anything else is silently dropped from the
 *  batch with a summary alert so we don't open a million error dialogs
 *  when the user multi-selects a folder of mixed files. */
const IMPORT_EXTS = ["csv", "json", "geojson"] as const;

interface Props {
  site: Site;
}

/** Browse / edit / create / delete files under `<site>/data/`. CSV
 *  files render as a spreadsheet grid; everything else opens in
 *  CodeMirror with the right language extension. */
export function DataLibrary({ site }: Props) {
  const queryClient = useQueryClient();

  const list = useQuery<DataFile[]>({
    queryKey: ["data", site.id],
    queryFn: () => tauri.dataList(site.id),
  });

  const [activePath, setActivePath] = useState<string | null>(null);
  const [dropFlash, setDropFlash] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-select the first file once the list lands so the editor side
  // isn't a blank rectangle staring at the user.
  useEffect(() => {
    if (activePath === null && list.data && list.data.length > 0) {
      setActivePath(list.data[0].relPath);
    }
  }, [list.data, activePath]);

  const file = useMemo(
    () => list.data?.find((f) => f.relPath === activePath) ?? null,
    [list.data, activePath],
  );

  const refreshList = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ["data", site.id] });
  }, [queryClient, site.id]);

  // OS drag-drop into the panel: claim the region while mounted; on a
  // matching drop, import every file with an accepted extension into
  // `data/`, refresh the listing, focus the first newly imported file.
  // Mismatched files are surfaced once at the end so a folder of mixed
  // content doesn't spawn a dialog stampede.
  useEffect(() => {
    return registerDropRegion({
      match: rectMatcher(() => containerRef.current),
      handle: async (paths) => {
        const accepted: string[] = [];
        const skipped: string[] = [];
        for (const p of paths) {
          const ext = p.split(/[\\/]/).pop()?.split(".").pop()?.toLowerCase();
          if (ext && (IMPORT_EXTS as readonly string[]).includes(ext)) {
            accepted.push(p);
          } else {
            skipped.push(p);
          }
        }
        if (accepted.length === 0) {
          if (skipped.length > 0) {
            alert(
              `Only .csv, .json and .geojson files can be imported into data/. Skipped:\n${skipped
                .map((p) => "  " + basename(p))
                .join("\n")}`,
            );
          }
          return;
        }
        let firstImported: DataFile | null = null;
        const failures: string[] = [];
        for (const source of accepted) {
          try {
            const file = await tauri.dataImport(site.id, source);
            if (!firstImported) firstImported = file;
          } catch (e) {
            failures.push(`${basename(source)}: ${describeError(e)}`);
          }
        }
        refreshList();
        if (firstImported) {
          setActivePath(firstImported.relPath);
          setDropFlash(true);
          setTimeout(() => setDropFlash(false), 600);
        }
        if (failures.length > 0 || skipped.length > 0) {
          const lines: string[] = [];
          if (skipped.length > 0) {
            lines.push(
              `Skipped (unsupported extension): ${skipped
                .map(basename)
                .join(", ")}`,
            );
          }
          if (failures.length > 0) {
            lines.push("Failed:");
            lines.push(...failures);
          }
          alert(lines.join("\n"));
        }
      },
    });
  }, [site.id, refreshList]);

  const createFile = useMutation({
    mutationFn: async () => {
      const proposed = window.prompt(
        "New data file path (e.g. team.csv, settings.json, products/sku.csv)",
      );
      if (!proposed) return null;
      const cleaned = proposed.trim();
      if (!cleaned) return null;
      return tauri.dataCreate(site.id, cleaned);
    },
    onSuccess: (created) => {
      if (!created) return;
      refreshList();
      setActivePath(created.relPath);
    },
    onError: (e) => alert(describeError(e)),
  });

  const removeFile = useMutation({
    mutationFn: (target: DataFile) => tauri.dataDelete(site.id, target.relPath),
    onSuccess: (_void, target) => {
      refreshList();
      setActivePath((cur) => (cur === target.relPath ? null : cur));
    },
    onError: (e) => alert(describeError(e)),
  });

  return (
    <div
      ref={containerRef}
      className={cn(
        "flex h-full overflow-hidden transition-shadow",
        dropFlash && "shadow-[inset_0_0_0_3px_hsl(var(--primary))]",
      )}
    >
      <aside className="flex w-60 shrink-0 flex-col border-r bg-muted/20">
        <header className="flex items-center justify-between border-b px-3 py-2">
          <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
            data/
          </span>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="size-6"
            onClick={() => createFile.mutate()}
            disabled={createFile.isPending}
            title="New data file"
            aria-label="New data file"
          >
            <Plus className="size-3.5" />
          </Button>
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
              No files in <code>data/</code> yet.
            </p>
          )}
          <ul className="space-y-0.5">
            {list.data?.map((f) => {
              const Icon = iconFor(f.format);
              const isActive = f.relPath === activePath;
              return (
                <li key={f.id} className="group flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => setActivePath(f.relPath)}
                    className={cn(
                      "flex flex-1 items-center gap-2 truncate rounded px-2 py-1 text-left text-xs hover:bg-accent",
                      isActive && "bg-accent font-medium",
                    )}
                    title={f.relPath}
                  >
                    <Icon className="size-3.5 shrink-0 text-muted-foreground" />
                    <span className="flex-1 truncate font-mono">
                      {f.relPath}
                    </span>
                  </button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-6 opacity-0 group-hover:opacity-100"
                    onClick={() => {
                      if (confirm(`Delete data/${f.relPath}?`)) {
                        removeFile.mutate(f);
                      }
                    }}
                    aria-label={`Delete ${f.relPath}`}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </li>
              );
            })}
          </ul>
        </div>
      </aside>

      <section className="flex flex-1 flex-col overflow-hidden">
        {file ? (
          <DataFileEditor key={file.relPath} site={site} file={file} />
        ) : (
          <div className="flex flex-1 items-center justify-center text-xs text-muted-foreground">
            Select a data file or click + to create one.
          </div>
        )}
      </section>
    </div>
  );
}

function DataFileEditor({ site, file }: { site: Site; file: DataFile }) {
  const queryClient = useQueryClient();
  const queryKey = ["data-file", site.id, file.relPath];

  const content = useQuery<DataFileContent>({
    queryKey,
    queryFn: () => tauri.dataRead(site.id, file.relPath),
  });

  const [draft, setDraft] = useState<string | null>(null);
  // Re-seed when the loaded text changes; once seeded, in-flight edits
  // are owned by `draft` and not bounced by re-renders.
  useEffect(() => {
    if (content.data && draft === null) setDraft(content.data.text);
  }, [content.data, draft]);

  const save = useMutation({
    mutationFn: (text: string) => tauri.dataWrite(site.id, file.relPath, text),
    onSuccess: (next) => {
      queryClient.setQueryData(queryKey, next);
      // Also refresh the listing so size changes show up if we ever
      // surface them; cheap.
      queryClient.invalidateQueries({ queryKey: ["data", site.id] });
      setDraft(next.text);
    },
    onError: (e) => alert(describeError(e)),
  });

  if (content.isPending) {
    return <p className="px-6 py-10 text-sm text-muted-foreground">Loading…</p>;
  }
  if (content.isError) {
    return (
      <p className="px-6 py-10 text-sm text-destructive">
        {describeError(content.error)}
      </p>
    );
  }
  if (draft === null) return null;

  const dirty = draft !== (content.data?.text ?? "");

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between gap-3 border-b px-4 py-2">
        <p
          className="truncate font-mono text-xs text-muted-foreground"
          title={file.path}
        >
          data/{file.relPath} · {file.format.toUpperCase()}
        </p>
        <Button
          type="button"
          size="sm"
          disabled={!dirty || save.isPending}
          onClick={() => save.mutate(draft)}
        >
          <Save className="size-3.5" />
          {save.isPending ? "Saving…" : "Save"}
        </Button>
      </header>
      <div className="flex-1 overflow-hidden">
        {file.format === "csv" ? (
          <CsvGrid value={draft} onChange={setDraft} />
        ) : (
          <BodyEditor
            value={draft}
            onChange={setDraft}
            language={file.format === "json" ? "json" : "markdown"}
          />
        )}
      </div>
    </div>
  );
}

function basename(p: string): string {
  const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return idx >= 0 ? p.slice(idx + 1) : p;
}

function iconFor(format: DataFile["format"]) {
  switch (format) {
    case "csv":
      return FileSpreadsheet;
    case "json":
      return FileJson;
    default:
      return FileText;
  }
}
