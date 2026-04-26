import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronLeft, Eye, EyeOff } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { describeError, tauri, type Site } from "@/lib/tauri";
import { useWorkspaceStore } from "@/store/workspace";
import { ContentTree } from "@/features/content-tree/ContentTree";
import { EditorView } from "@/features/editor/EditorView";
import { GitPanel } from "@/features/git/GitPanel";
import { MediaLibrary } from "@/features/media/MediaLibrary";
import { PreviewPane } from "@/features/preview/PreviewPane";
import { SiteSettingsPanel } from "@/features/site-settings/SiteSettingsPanel";
import { ThemeSettingsPanel } from "@/features/theme-settings/ThemeSettingsPanel";
import { ThemeToggle } from "@/features/theme-mode/ThemeToggle";

const KIND_LABEL: Record<Site["detection"]["kind"], string> = {
  hugoToml: "hugo.toml",
  hugoYaml: "hugo.yaml",
  hugoJson: "hugo.json",
  configToml: "config.toml (deprecated)",
  configYaml: "config.yaml (deprecated)",
  configJson: "config.json (deprecated)",
  defaultDirectory: "config/_default/",
};

interface Props {
  site: Site;
}

export function SiteShell({ site }: Props) {
  const queryClient = useQueryClient();
  const setActiveSite = useWorkspaceStore((s) => s.setActiveSite);
  const selection = useWorkspaceStore((s) => s.selection);
  const clearSelection = useWorkspaceStore((s) => s.selectContent);
  // Preview pane visibility is per-window UI state — the lifecycle of the
  // hugo process itself lives in the preview store and survives toggling
  // this pane (so hiding the iframe does not kill the server).
  const [previewOpen, setPreviewOpen] = useState(false);

  const back = useMutation({
    mutationFn: () => tauri.workspaceClearActive(),
    onSuccess: () => {
      setActiveSite(null);
      queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
    onError: (err) => alert(describeError(err)),
  });

  const gridCols = previewOpen
    ? "grid-cols-[280px_1fr_minmax(380px,1fr)]"
    : "grid-cols-[280px_1fr]";

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b px-6 py-3">
        <div className="flex items-center gap-3">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => back.mutate()}
            disabled={back.isPending}
          >
            <ChevronLeft className="size-4" />
            Workspace
          </Button>
          <div className="border-l pl-3">
            <h1 className="text-base font-semibold leading-tight">
              {site.name}
            </h1>
            <p
              className="truncate text-xs text-muted-foreground"
              title={site.rootPath}
            >
              {site.rootPath}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {selection && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => clearSelection(null)}
            >
              Site settings
            </Button>
          )}
          <Button
            size="sm"
            variant={previewOpen ? "default" : "outline"}
            onClick={() => setPreviewOpen((v) => !v)}
          >
            {previewOpen ? (
              <EyeOff className="size-4" />
            ) : (
              <Eye className="size-4" />
            )}
            Preview
          </Button>
          <ThemeToggle />
          <span className="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            config: {KIND_LABEL[site.detection.kind]}
          </span>
        </div>
      </header>

      <div className={`grid flex-1 ${gridCols} overflow-hidden`}>
        <aside className="flex h-full flex-col overflow-hidden border-r bg-muted/20">
          <ContentTree site={site} />
        </aside>

        <main className="flex-1 overflow-hidden border-r">
          {selection ? (
            <EditorView site={site} selection={selection} />
          ) : (
            <Tabs
              defaultValue="site"
              className="flex h-full flex-col overflow-hidden"
            >
              <div className="border-b px-6 py-2">
                <TabsList>
                  <TabsTrigger value="site">Site</TabsTrigger>
                  <TabsTrigger value="theme">Theme</TabsTrigger>
                  <TabsTrigger value="media">Media</TabsTrigger>
                  <TabsTrigger value="git">Git</TabsTrigger>
                </TabsList>
              </div>
              <TabsContent value="site" className="mt-0 flex-1 overflow-auto">
                <SiteSettingsPanel site={site} />
              </TabsContent>
              <TabsContent value="theme" className="mt-0 flex-1 overflow-auto">
                <ThemeSettingsPanel site={site} />
              </TabsContent>
              <TabsContent
                value="media"
                className="mt-0 flex flex-1 flex-col overflow-hidden"
              >
                <MediaLibrary site={site} />
              </TabsContent>
              <TabsContent value="git" className="mt-0 flex-1 overflow-auto">
                <GitPanel site={site} />
              </TabsContent>
            </Tabs>
          )}
        </main>

        {previewOpen && (
          <aside className="flex h-full flex-col overflow-hidden">
            <PreviewPane site={site} onClose={() => setPreviewOpen(false)} />
          </aside>
        )}
      </div>
    </div>
  );
}
