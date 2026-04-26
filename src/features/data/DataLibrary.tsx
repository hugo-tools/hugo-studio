import { useCallback, useEffect, useMemo, useState } from "react";
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
import {
  describeError,
  tauri,
  type DataFile,
  type DataFileContent,
  type Site,
} from "@/lib/tauri";
import { BodyEditor } from "@/features/editor/BodyEditor";
import { CsvGrid } from "./CsvGrid";

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
    <div className="flex h-full overflow-hidden">
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
