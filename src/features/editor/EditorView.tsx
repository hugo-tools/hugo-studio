import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { ImagePlus, Save, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  describeError,
  tauri,
  type AssetContext,
  type AssetRef,
  type ContentEditPayload,
  type JsonValue,
  type Site,
} from "@/lib/tauri";
import { useWorkspaceStore, type EditorSelection } from "@/store/workspace";
import { tryDispatchDrop } from "@/lib/dnd/regions";
import { AssetImportDialog } from "@/features/assets/AssetImportDialog";
import { BundleAssetsPanel } from "@/features/assets/BundleAssetsPanel";
import { MediaPickerDialog } from "@/features/media/MediaPickerDialog";
import { FrontMatterForm } from "./FrontMatterForm";
import { BodyEditor, type BodyEditorHandle } from "./BodyEditor";
import { RichEditor } from "./RichEditor";

type BodyFormat = ContentEditPayload["bodyFormat"];

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

  const [fm, setFm] = useState<Record<string, JsonValue>>({});
  const [body, setBody] = useState("");
  const [savedFlash, setSavedFlash] = useState(false);
  const editorRef = useRef<BodyEditorHandle>(null);

  // Asset import dialog state — opened when the user drops files on the
  // editor surface. The dropped paths are buffered until the dialog
  // resolves, then routed through assetImport one by one.
  const [pendingFiles, setPendingFiles] = useState<string[]>([]);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [mediaPickerOpen, setMediaPickerOpen] = useState(false);

  useEffect(() => {
    if (!doc.data) return;
    setFm((doc.data.frontMatter as Record<string, JsonValue>) ?? {});
    setBody(doc.data.body);
  }, [doc.data]);

  const bundleAvailable = useMemo(
    () => /(?:^|[\\/])(_?index)\.\w+$/i.test(selection.path),
    [selection.path],
  );
  const bundleContentId = bundleAvailable ? selection.path : null;
  const bundleLabel = bundleAvailable
    ? selection.id.replace(/\/[^/]+$/, "/") || selection.id
    : null;

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | null = null;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        if (!event.payload.paths.length) return;
        // Let registered regions (e.g. the Media library / picker) claim
        // the drop first based on cursor coordinates. Only fall back to
        // the import dialog when nothing more specific applies.
        if (
          tryDispatchDrop(event.payload.position, event.payload.paths.slice())
        ) {
          return;
        }
        setPendingFiles(event.payload.paths.slice());
        setDialogOpen(true);
      })
      .then((u) => {
        if (cancelled) u();
        else unlisten = u;
      });
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  const importAssets = useMutation({
    mutationFn: async (context: AssetContext) => {
      const out: AssetRef[] = [];
      for (const source of pendingFiles) {
        const resolved =
          context.kind === "bundle" && context.contentId === "__current__"
            ? { kind: "bundle" as const, contentId: bundleContentId ?? "" }
            : context;
        out.push(await tauri.assetImport(site.id, source, resolved));
      }
      return out;
    },
    onSuccess: (refs) => {
      setDialogOpen(false);
      setPendingFiles([]);
      for (const a of refs)
        editorRef.current?.insertAtCursor(
          linkFor(a, doc.data?.bodyFormat ?? "markdown"),
        );
      queryClient.invalidateQueries({
        queryKey: ["assets", site.id, bundleContentId],
      });
    },
    onError: (e) => alert(describeError(e)),
  });

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

  const showAssets = bundleAvailable;
  const gridCols = showAssets
    ? "lg:grid-cols-[minmax(0,1fr)_220px]"
    : "lg:grid-cols-1";

  const bodyFormat = doc.data.bodyFormat;
  const isHtml = bodyFormat === "html";
  const titleValue = (fm.title as string | undefined) ?? "";

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-col gap-2 border-b px-6 py-3">
        <div className="flex items-center justify-between gap-3">
          <p
            className="truncate font-mono text-xs text-muted-foreground"
            title={doc.data.path}
          >
            {doc.data.path} · {doc.data.format.toUpperCase()} front-matter ·{" "}
            {isHtml ? "HTML" : "Markdown"} body
          </p>
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
              variant="outline"
              onClick={() => setMediaPickerOpen(true)}
              title="Browse media and insert a link at the caret"
            >
              <ImagePlus className="size-4" />
              Insert media
            </Button>
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
        </div>
        <Input
          type="text"
          value={titleValue}
          onChange={(e) => {
            const next = e.target.value;
            setFm((prev) => {
              const copy = { ...prev };
              if (next === "") delete copy.title;
              else copy.title = next;
              return copy;
            });
          }}
          placeholder="Page title"
          className="h-9 text-base font-semibold"
        />
      </header>

      <div className={`grid flex-1 grid-cols-1 overflow-hidden ${gridCols}`}>
        {/* Re-mount when the loaded file changes so the format-aware
            defaultValue actually takes effect (and so `defaultValue`
            differences across files don't get pinned by the first
            mount). HTML pages typically carry the bulk of their
            content in the body, so we land there directly; markdown
            pages start on Front matter as before. */}
        <Tabs
          key={`${selection.path}-${bodyFormat}`}
          defaultValue={isHtml ? "body" : "frontmatter"}
          className="flex h-full flex-col overflow-hidden"
        >
          <div className="border-b px-6 py-2">
            <TabsList>
              <TabsTrigger value="frontmatter">Front matter</TabsTrigger>
              <TabsTrigger value="body">Body</TabsTrigger>
              {!isHtml && <TabsTrigger value="rich">Rich</TabsTrigger>}
            </TabsList>
          </div>
          <TabsContent
            value="frontmatter"
            className="mt-0 flex-1 overflow-auto p-6"
          >
            <FrontMatterForm
              schema={doc.data.schema}
              values={fm}
              onChange={setFm}
            />
          </TabsContent>
          <TabsContent
            value="body"
            className="mt-0 flex h-full flex-1 flex-col overflow-hidden"
          >
            <BodyEditor
              ref={editorRef}
              value={body}
              onChange={setBody}
              language={isHtml ? "html" : "markdown"}
            />
          </TabsContent>
          {!isHtml && (
            <TabsContent
              value="rich"
              className="mt-0 flex h-full flex-1 flex-col overflow-hidden"
            >
              <div className="border-b bg-muted/30 px-4 py-1.5 text-[10px] text-muted-foreground">
                Visual editor (Milkdown). Hugo shortcodes pass through as raw
                text. Switching back to Body may canonicalise the markdown (e.g.
                `*foo*` ↔ `_foo_`).
              </div>
              {/* Re-mount whenever the loaded document path changes so a
                  fresh body string seeds the editor. While the user stays
                  on this content, Crepe owns the in-flight state. */}
              <div className="flex-1 overflow-hidden">
                <RichEditor
                  key={selection.path}
                  value={body}
                  onChange={setBody}
                />
              </div>
            </TabsContent>
          )}
        </Tabs>
        {showAssets && (
          <BundleAssetsPanel
            siteId={site.id}
            contentId={bundleContentId}
            onInsertLink={(a) =>
              editorRef.current?.insertAtCursor(linkFor(a, bodyFormat))
            }
          />
        )}
      </div>

      <AssetImportDialog
        open={dialogOpen}
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setPendingFiles([]);
        }}
        files={pendingFiles}
        bundleAvailable={bundleAvailable}
        bundleLabel={bundleLabel}
        onConfirm={(ctx) => importAssets.mutate(ctx)}
      />

      <MediaPickerDialog
        open={mediaPickerOpen}
        onOpenChange={setMediaPickerOpen}
        site={site}
        bundleContentId={bundleContentId}
        bundleLabel={bundleLabel}
        onSelect={(a) =>
          editorRef.current?.insertAtCursor(linkFor(a, bodyFormat))
        }
      />
    </div>
  );
}

function linkFor(asset: AssetRef, format: BodyFormat): string {
  const altSuggestion = asset.name.replace(/\.[^.]+$/, "");
  if (format === "html") {
    if (asset.kind === "image") {
      return `<img src="${asset.relativeLink}" alt="${escapeAttr(altSuggestion)}" />`;
    }
    return `<a href="${asset.relativeLink}">${escapeText(altSuggestion)}</a>`;
  }
  if (asset.kind === "image") {
    return `![${altSuggestion}](${asset.relativeLink})`;
  }
  return `[${altSuggestion}](${asset.relativeLink})`;
}

function escapeAttr(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
}

function escapeText(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
