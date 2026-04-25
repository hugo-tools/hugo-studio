import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Save, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  describeError,
  tauri,
  type ContentEditPayload,
  type JsonValue,
  type Site,
} from "@/lib/tauri";
import { useWorkspaceStore, type EditorSelection } from "@/store/workspace";
import { FrontMatterForm } from "./FrontMatterForm";
import { BodyEditor } from "./BodyEditor";

interface Props {
  site: Site;
  selection: EditorSelection;
}

export function EditorView({ site, selection }: Props) {
  const queryClient = useQueryClient();
  const clearSelection = useWorkspaceStore((s) => s.selectContent);

  const doc = useQuery<ContentEditPayload>({
    queryKey: ["content-doc", site.id, selection.path],
    queryFn: () => tauri.contentGet(site.id, selection.path),
  });

  // Local working copy. We seed it from `doc.data` on first arrival and
  // whenever the user picks a different file.
  const [fm, setFm] = useState<Record<string, JsonValue>>({});
  const [body, setBody] = useState("");
  const [savedFlash, setSavedFlash] = useState(false);

  useEffect(() => {
    if (!doc.data) return;
    setFm((doc.data.frontMatter as Record<string, JsonValue>) ?? {});
    setBody(doc.data.body);
  }, [doc.data]);

  const dirty =
    !!doc.data &&
    (JSON.stringify(fm) !== JSON.stringify(doc.data.frontMatter) ||
      body !== doc.data.body);

  const save = useMutation({
    mutationFn: () =>
      tauri.contentSave(site.id, selection.path, fm as JsonValue, body),
    onSuccess: (next) => {
      queryClient.setQueryData(["content-doc", site.id, selection.path], next);
      queryClient.invalidateQueries({ queryKey: ["content", site.id] });
      setSavedFlash(true);
      setTimeout(() => setSavedFlash(false), 1800);
    },
  });

  if (doc.isPending) {
    return (
      <div className="px-6 py-10 text-sm text-muted-foreground">
        Loading {selection.path}…
      </div>
    );
  }
  if (doc.isError || !doc.data) {
    return (
      <div className="px-6 py-10 text-sm text-destructive">
        Failed to load: {describeError(doc.error)}
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between gap-3 border-b px-6 py-3">
        <div className="min-w-0">
          <h2 className="truncate text-base font-semibold">
            {(fm.title as string | undefined) ??
              selection.title ??
              selection.id}
          </h2>
          <p
            className="truncate font-mono text-xs text-muted-foreground"
            title={doc.data.path}
          >
            {doc.data.path} · {doc.data.format.toUpperCase()} front-matter
          </p>
        </div>
        <div className="flex items-center gap-3">
          {savedFlash && (
            <span className="text-xs text-muted-foreground">Saved.</span>
          )}
          {save.isError && (
            <span className="text-xs text-destructive">
              {describeError(save.error)}
            </span>
          )}
          <Button
            type="button"
            size="sm"
            disabled={!dirty || save.isPending}
            onClick={() => save.mutate()}
          >
            <Save className="size-4" />
            {save.isPending ? "Saving…" : "Save"}
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={() => clearSelection(null)}
            aria-label="Close editor"
          >
            <X className="size-4" />
          </Button>
        </div>
      </header>

      <div className="grid flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <section className="overflow-auto border-r p-6">
          <FrontMatterForm
            schema={doc.data.schema}
            values={fm}
            onChange={setFm}
          />
        </section>
        <section className="flex h-full flex-col overflow-hidden">
          <div className="border-b px-6 py-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
            Body — Markdown
          </div>
          <div className="flex-1 overflow-hidden">
            <BodyEditor value={body} onChange={setBody} />
          </div>
        </section>
      </div>
    </div>
  );
}
