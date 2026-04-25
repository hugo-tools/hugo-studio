import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronLeft } from "lucide-react";

import { Button } from "@/components/ui/button";
import { describeError, tauri, type Site } from "@/lib/tauri";
import { useWorkspaceStore } from "@/store/workspace";
import { ContentTree } from "@/features/content-tree/ContentTree";
import { EditorView } from "@/features/editor/EditorView";
import { SiteSettingsPanel } from "@/features/site-settings/SiteSettingsPanel";

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

  const back = useMutation({
    mutationFn: () => tauri.workspaceClearActive(),
    onSuccess: () => {
      setActiveSite(null);
      queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
    onError: (err) => alert(describeError(err)),
  });

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
          <span className="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            config: {KIND_LABEL[site.detection.kind]}
          </span>
        </div>
      </header>

      <div className="grid flex-1 grid-cols-[280px_1fr] overflow-hidden">
        <aside className="flex h-full flex-col overflow-hidden border-r bg-muted/20">
          <ContentTree site={site} />
        </aside>

        <main className="flex-1 overflow-hidden">
          {selection ? (
            <EditorView site={site} selection={selection} />
          ) : (
            <div className="h-full overflow-auto">
              <SiteSettingsPanel site={site} />
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
