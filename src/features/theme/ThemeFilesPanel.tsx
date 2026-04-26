import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  FileCode,
  FileText,
  File as FileIcon,
  Palette,
  Save,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  describeError,
  tauri,
  type Site,
  type ThemeFileContent,
  type ThemeFilesIndex,
} from "@/lib/tauri";
import { BodyEditor, type BodyLanguage } from "@/features/editor/BodyEditor";

interface Props {
  site: Site;
}

type ThemeFileFormat = NonNullable<ThemeFilesIndex["files"][number]>["format"];

/** Browse and edit raw source files inside the active theme. Layouts,
 *  partials, SCSS, JS — anything under `themes/<name>/`. Edits write
 *  through atomically; the panel deliberately doesn't offer a delete
 *  control because nuking a theme template usually breaks the build
 *  in non-obvious ways. */
export function ThemeFilesPanel({ site }: Props) {
  const index = useQuery<ThemeFilesIndex>({
    queryKey: ["theme-files", site.id],
    queryFn: () => tauri.themeFilesList(site.id),
  });

  const [activePath, setActivePath] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  // Drop the active selection if the next index doesn't include it
  // (e.g. theme switched out from under us).
  useEffect(() => {
    if (!index.data) return;
    if (activePath && !index.data.files.some((f) => f.relPath === activePath)) {
      setActivePath(null);
    }
  }, [index.data, activePath]);

  const groups = useMemo(
    () => groupFiles(index.data?.files ?? [], filter),
    [index.data, filter],
  );

  const file = useMemo(
    () => index.data?.files.find((f) => f.relPath === activePath) ?? null,
    [index.data, activePath],
  );

  if (index.isPending) {
    return <p className="px-6 py-10 text-sm text-muted-foreground">Loading…</p>;
  }
  if (index.isError) {
    return (
      <p className="px-6 py-10 text-sm text-destructive">
        {describeError(index.error)}
      </p>
    );
  }
  if (!index.data?.themePath) {
    return (
      <div className="space-y-3 px-6 py-10 text-sm text-muted-foreground">
        <p>
          No active theme found. Set one via the Site config (e.g.
          <code className="ml-1 rounded bg-muted px-1 py-0.5 font-mono text-xs">
            theme = "ace"
          </code>
          ) and the files will show up here.
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden">
      <aside className="flex w-72 shrink-0 flex-col border-r bg-muted/20">
        <header className="flex flex-col gap-2 border-b px-3 py-2">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-xs font-medium uppercase tracking-wider text-muted-foreground">
              {index.data.themeName ?? "theme"}
            </span>
            <span className="shrink-0 text-[10px] text-muted-foreground">
              {index.data.files.length}
            </span>
          </div>
          <Input
            type="search"
            placeholder="Filter…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="h-7 text-xs"
          />
        </header>
        <div className="flex-1 overflow-auto px-1 py-2">
          {groups.length === 0 && (
            <p className="px-2 text-xs text-muted-foreground">No matches.</p>
          )}
          {groups.map((g) => (
            <div key={g.category} className="mb-3">
              <div className="px-2 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                {g.category || "(root)"}
              </div>
              <ul className="space-y-0.5">
                {g.files.map((f) => {
                  const Icon = iconFor(f.format);
                  const isActive = f.relPath === activePath;
                  return (
                    <li key={f.relPath}>
                      <button
                        type="button"
                        onClick={() => setActivePath(f.relPath)}
                        className={cn(
                          "flex w-full items-center gap-2 truncate rounded px-2 py-1 text-left text-xs hover:bg-accent",
                          isActive && "bg-accent font-medium",
                        )}
                        title={f.relPath}
                      >
                        <Icon className="size-3.5 shrink-0 text-muted-foreground" />
                        <span className="flex-1 truncate font-mono">
                          {trimCategory(f.relPath, g.category)}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>
      </aside>

      <section className="flex flex-1 flex-col overflow-hidden">
        {file ? (
          <ThemeFileEditor
            key={file.relPath}
            site={site}
            relPath={file.relPath}
            absPath={file.path}
            format={file.format}
          />
        ) : (
          <div className="flex flex-1 items-center justify-center text-xs text-muted-foreground">
            Select a file to edit. Touching theme templates can break the site —
            proceed with care.
          </div>
        )}
      </section>
    </div>
  );
}

function ThemeFileEditor({
  site,
  relPath,
  absPath,
  format,
}: {
  site: Site;
  relPath: string;
  absPath: string;
  format: ThemeFileFormat;
}) {
  const queryClient = useQueryClient();
  const queryKey = ["theme-file", site.id, relPath];

  const content = useQuery<ThemeFileContent>({
    queryKey,
    queryFn: () => tauri.themeFileRead(site.id, relPath),
  });

  const [draft, setDraft] = useState<string | null>(null);
  useEffect(() => {
    if (content.data && draft === null) setDraft(content.data.text);
  }, [content.data, draft]);

  const save = useMutation({
    mutationFn: (text: string) => tauri.themeFileWrite(site.id, relPath, text),
    onSuccess: (next) => {
      queryClient.setQueryData(queryKey, next);
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
      <header className="flex flex-col gap-1 border-b px-4 py-2">
        <div className="flex items-center justify-between gap-3">
          <p
            className="truncate font-mono text-xs text-muted-foreground"
            title={absPath}
          >
            {relPath} · {format.toUpperCase()}
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
        </div>
        <p className="flex items-start gap-1.5 text-[10px] text-amber-700 dark:text-amber-400">
          <AlertTriangle className="mt-px size-3 shrink-0" />
          <span>
            Editing files in a vendored theme makes it diverge from the upstream
            — re-vendoring or upgrading will overwrite your changes.
          </span>
        </p>
      </header>
      <div className="flex-1 overflow-hidden">
        <BodyEditor
          value={draft}
          onChange={setDraft}
          language={editorLanguageFor(format)}
        />
      </div>
    </div>
  );
}

function editorLanguageFor(format: ThemeFileFormat): BodyLanguage {
  switch (format) {
    case "html":
      return "html";
    case "json":
      return "json";
    case "css":
    case "scss":
      return "scss";
    case "js":
      return "javascript";
    case "ts":
      return "typescript";
    default:
      return "markdown";
  }
}

function iconFor(format: ThemeFileFormat) {
  switch (format) {
    case "css":
    case "scss":
      return Palette;
    case "html":
    case "js":
    case "ts":
    case "json":
      return FileCode;
    case "markdown":
    case "yaml":
    case "toml":
      return FileText;
    default:
      return FileIcon;
  }
}

interface Group {
  category: string;
  files: ThemeFilesIndex["files"];
}

function groupFiles(files: ThemeFilesIndex["files"], filter: string): Group[] {
  const f = filter.trim().toLowerCase();
  const filtered = f
    ? files.filter((file) => file.relPath.toLowerCase().includes(f))
    : files;
  const buckets = new Map<string, ThemeFilesIndex["files"]>();
  for (const file of filtered) {
    const bucket = buckets.get(file.category) ?? [];
    bucket.push(file);
    buckets.set(file.category, bucket);
  }
  // Stable order: root first, then alphabetic.
  return Array.from(buckets.entries())
    .sort(([a], [b]) => {
      if (a === "" && b !== "") return -1;
      if (b === "" && a !== "") return 1;
      return a.localeCompare(b);
    })
    .map(([category, files]) => ({ category, files }));
}

function trimCategory(rel: string, category: string): string {
  if (!category) return rel;
  const prefix = category + "/";
  return rel.startsWith(prefix) ? rel.slice(prefix.length) : rel;
}
