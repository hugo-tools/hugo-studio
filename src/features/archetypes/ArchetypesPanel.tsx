import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  FileText,
  FolderClosed,
  FolderTree,
  Plus,
  Save,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { cn } from "@/lib/utils";
import {
  describeError,
  tauri,
  type Archetype,
  type ArchetypeContent,
  type ArchetypeKind,
  type ContentKind,
  type Site,
} from "@/lib/tauri";
import { BodyEditor } from "@/features/editor/BodyEditor";

interface Props {
  site: Site;
}

/** Browse / edit / create / delete files under `<site>/archetypes/`.
 *  Hugo uses these as templates when the user creates a new piece of
 *  content via `hugo new`. The new-content wizard already pulls them
 *  in; this panel lets editors curate the templates themselves. */
export function ArchetypesPanel({ site }: Props) {
  const queryClient = useQueryClient();

  const list = useQuery<Archetype[]>({
    queryKey: ["archetypes", site.id],
    queryFn: () => tauri.contentArchetypes(site.id),
  });

  const [activeName, setActiveName] = useState<string | null>(null);
  const [newOpen, setNewOpen] = useState(false);

  // Auto-select the first archetype once the list lands so the editor
  // pane isn't a blank rectangle.
  useEffect(() => {
    if (activeName === null && list.data && list.data.length > 0) {
      setActiveName(list.data[0].name);
    }
  }, [list.data, activeName]);

  // Drop the active selection if the next list doesn't include it
  // (e.g. the user just deleted the active archetype).
  useEffect(() => {
    if (
      activeName &&
      list.data &&
      !list.data.some((a) => a.name === activeName)
    ) {
      setActiveName(list.data[0]?.name ?? null);
    }
  }, [list.data, activeName]);

  const groups = useMemo(() => groupByKind(list.data ?? []), [list.data]);

  const removeArch = useMutation({
    mutationFn: (name: string) => tauri.archetypeDelete(site.id, name),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["archetypes", site.id] }),
    onError: (e) => alert(describeError(e)),
  });

  const active = list.data?.find((a) => a.name === activeName) ?? null;

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between gap-3 border-b px-6 py-3">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold leading-tight">Archetypes</h2>
          <p className="truncate text-xs text-muted-foreground">
            Hugo content templates under <code>archetypes/</code>. Used by "New
            content" + <code>hugo new</code>.
          </p>
        </div>
        <Button type="button" size="sm" onClick={() => setNewOpen(true)}>
          <Plus className="size-3.5" />
          New archetype
        </Button>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <aside className="flex w-60 shrink-0 flex-col overflow-auto border-r bg-muted/20 py-2">
          {list.isPending && (
            <p className="px-4 text-xs text-muted-foreground">Loading…</p>
          )}
          {list.isError && (
            <p className="px-4 text-xs text-destructive">
              {describeError(list.error)}
            </p>
          )}
          {list.data && list.data.length === 0 && (
            <p className="px-4 text-xs text-muted-foreground">
              No archetypes yet. Click "New archetype" to add one.
            </p>
          )}
          {groups.map((g) => (
            <div key={g.kind} className="mb-3">
              <div className="px-4 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                {labelForKind(g.kind)}
              </div>
              <ul className="space-y-0.5 px-1">
                {g.items.map((arch) => {
                  const Icon = iconForKind(arch.kind);
                  const isActive = arch.name === activeName;
                  return (
                    <li
                      key={arch.path}
                      className="group flex items-center gap-1"
                    >
                      <button
                        type="button"
                        onClick={() => setActiveName(arch.name)}
                        className={cn(
                          "flex flex-1 items-center gap-2 truncate rounded px-2 py-1 text-left text-xs hover:bg-accent",
                          isActive && "bg-accent font-medium",
                        )}
                        title={arch.path}
                      >
                        <Icon className="size-3.5 shrink-0 text-muted-foreground" />
                        <span className="flex-1 truncate font-mono">
                          {arch.name}
                        </span>
                      </button>
                      <Button
                        type="button"
                        size="icon"
                        variant="ghost"
                        className="size-6 text-destructive opacity-0 hover:text-destructive group-hover:opacity-100"
                        onClick={() => {
                          if (
                            confirm(
                              `Delete archetype "${arch.name}"? This removes the ${
                                arch.kind === "singlePage"
                                  ? "file"
                                  : "whole bundle directory"
                              }.`,
                            )
                          ) {
                            removeArch.mutate(arch.name);
                          }
                        }}
                        aria-label={`Delete ${arch.name}`}
                      >
                        <Trash2 className="size-3" />
                      </Button>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </aside>

        <section className="flex flex-1 flex-col overflow-hidden">
          {active ? (
            <ArchetypeEditor key={active.path} site={site} archetype={active} />
          ) : (
            <div className="flex flex-1 items-center justify-center text-xs text-muted-foreground">
              Select or create an archetype on the left.
            </div>
          )}
        </section>
      </div>

      <NewArchetypeDialog
        open={newOpen}
        onOpenChange={setNewOpen}
        existingNames={list.data?.map((a) => a.name) ?? []}
        onCreated={(arch) => {
          queryClient.invalidateQueries({
            queryKey: ["archetypes", site.id],
          });
          setActiveName(arch.name);
        }}
        site={site}
      />
    </div>
  );
}

function ArchetypeEditor({
  site,
  archetype,
}: {
  site: Site;
  archetype: Archetype;
}) {
  const queryClient = useQueryClient();
  const queryKey = ["archetype-content", site.id, archetype.name];

  const content = useQuery<ArchetypeContent>({
    queryKey,
    queryFn: () => tauri.archetypeRead(site.id, archetype.name),
  });

  const [draft, setDraft] = useState<string | null>(null);
  useEffect(() => {
    if (content.data && draft === null) setDraft(content.data.text);
  }, [content.data, draft]);

  const save = useMutation({
    mutationFn: (text: string) =>
      tauri.archetypeWrite(site.id, archetype.name, text),
    onSuccess: (next) => {
      queryClient.setQueryData(queryKey, next);
      setDraft(next.text);
    },
    onError: (e) => alert(describeError(e)),
  });

  if (content.isPending) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-muted-foreground">Loading…</p>
    );
  }
  if (content.isError) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-destructive">
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
            title={archetype.path}
          >
            archetypes/
            {archetype.name}
            {archetype.kind === "leafBundle"
              ? "/index.md"
              : archetype.kind === "branchBundle"
                ? "/_index.md"
                : ".md"}{" "}
            · {labelForKind(archetype.kind)}
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
        <p className="text-[10px] text-muted-foreground">
          Go-template substitutions like{" "}
          <code className="font-mono">{"{{ .Title }}"}</code>,{" "}
          <code className="font-mono">{"{{ .Date }}"}</code>,{" "}
          <code className="font-mono">{"{{ .Section }}"}</code> are filled in
          when content is created from this archetype.
        </p>
      </header>
      <div className="flex-1 overflow-hidden">
        <BodyEditor value={draft} onChange={setDraft} language="markdown" />
      </div>
    </div>
  );
}

function NewArchetypeDialog({
  open,
  onOpenChange,
  existingNames,
  site,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  existingNames: string[];
  site: Site;
  onCreated: (arch: ArchetypeContent) => void;
}) {
  const [name, setName] = useState("");
  const [kind, setKind] = useState<ArchetypeKind>("single");

  useEffect(() => {
    if (!open) {
      setName("");
      setKind("single");
    }
  }, [open]);

  const create = useMutation({
    mutationFn: () => tauri.archetypeCreate(site.id, name.trim(), kind, null),
    onSuccess: (arch) => {
      onCreated(arch);
      onOpenChange(false);
    },
    onError: (e) => alert(describeError(e)),
  });

  const trimmed = name.trim();
  const collision = existingNames.includes(trimmed);
  const invalid = trimmed === "" || /[\\/]/.test(trimmed) || collision;

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>New archetype</AlertDialogTitle>
          <AlertDialogDescription>
            Templates for new content. The name becomes the file stem (single
            page) or directory name (bundle); use the section name you want this
            archetype to apply to (e.g. <code>posts</code>), or{" "}
            <code>default</code> for the site-wide fallback.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-4 py-2">
          <label className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">Name</span>
            <Input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="posts, default, story-bundle…"
              autoFocus
            />
            {collision && (
              <span className="text-xs text-destructive">
                An archetype named "{trimmed}" already exists.
              </span>
            )}
            {trimmed !== "" && /[\\/]/.test(trimmed) && (
              <span className="text-xs text-destructive">
                Slashes aren't allowed in archetype names.
              </span>
            )}
          </label>

          <fieldset className="space-y-1.5">
            <legend className="text-sm font-medium">Kind</legend>
            {(
              [
                {
                  value: "single",
                  label: "Single page",
                  hint: "archetypes/<name>.md",
                },
                {
                  value: "leafBundle",
                  label: "Leaf bundle",
                  hint: "archetypes/<name>/index.md (with sibling assets)",
                },
                {
                  value: "branchBundle",
                  label: "Branch bundle (section)",
                  hint: "archetypes/<name>/_index.md (section index)",
                },
              ] as const
            ).map((opt) => (
              <label
                key={opt.value}
                className={cn(
                  "flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm",
                  kind === opt.value && "border-primary bg-primary/5",
                )}
              >
                <input
                  type="radio"
                  name="archetype-kind"
                  value={opt.value}
                  checked={kind === opt.value}
                  onChange={() => setKind(opt.value)}
                  className="mt-0.5 size-4"
                />
                <div>
                  <div className="font-medium">{opt.label}</div>
                  <div className="font-mono text-xs text-muted-foreground">
                    {opt.hint}
                  </div>
                </div>
              </label>
            ))}
          </fieldset>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            disabled={invalid || create.isPending}
            onClick={(e) => {
              e.preventDefault();
              if (invalid) return;
              create.mutate();
            }}
          >
            {create.isPending ? "Creating…" : "Create"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

interface KindGroup {
  kind: ContentKind;
  items: Archetype[];
}

function groupByKind(items: Archetype[]): KindGroup[] {
  const buckets = new Map<ContentKind, Archetype[]>();
  for (const it of items) {
    const bucket = buckets.get(it.kind) ?? [];
    bucket.push(it);
    buckets.set(it.kind, bucket);
  }
  // Stable display order: single page first (most common), then bundles.
  const order: ContentKind[] = [
    "singlePage",
    "leafBundle",
    "branchBundle",
    "section",
  ];
  return order
    .filter((k) => buckets.has(k))
    .map((kind) => ({ kind, items: buckets.get(kind)! }));
}

function labelForKind(kind: ContentKind): string {
  switch (kind) {
    case "singlePage":
      return "Single page";
    case "leafBundle":
      return "Leaf bundle";
    case "branchBundle":
      return "Branch bundle";
    case "section":
      return "Section";
  }
}

function iconForKind(kind: ContentKind) {
  switch (kind) {
    case "leafBundle":
      return FolderClosed;
    case "branchBundle":
      return FolderTree;
    default:
      return FileText;
  }
}
