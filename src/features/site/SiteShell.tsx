import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronLeft } from "lucide-react";

import { Button } from "@/components/ui/button";
import { tauri, describeError, type Site } from "@/lib/tauri";
import { useWorkspaceStore } from "@/store/workspace";

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

  const back = useMutation({
    mutationFn: () => tauri.workspaceClearActive(),
    onSuccess: () => {
      setActiveSite(null);
      queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
    onError: (err) => alert(describeError(err)),
  });

  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex items-center justify-between border-b px-6 py-4">
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
        <span className="rounded-full border px-3 py-1 text-xs text-muted-foreground">
          config: {KIND_LABEL[site.detection.kind]}
        </span>
      </header>

      <main className="flex-1 px-6 py-10">
        <div className="mx-auto max-w-2xl rounded-lg border border-dashed bg-muted/30 px-6 py-10 text-center">
          <h2 className="text-lg font-medium">Site opened</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            The content tree, front-matter editor and live preview land in
            milestones M3 – M6. For now, this surface confirms that the site was
            registered and detected correctly.
          </p>
          <dl className="mt-6 grid grid-cols-1 gap-3 text-left text-sm sm:grid-cols-2">
            <Field label="Site ID" value={site.id} mono />
            <Field label="Detection" value={KIND_LABEL[site.detection.kind]} />
            <Field label="Content root" value={site.contentRoot} mono />
            <Field label="Config path" value={site.detection.configPath} mono />
          </dl>
        </div>
      </main>
    </div>
  );
}

function Field({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-muted-foreground">
        {label}
      </dt>
      <dd
        className={mono ? "mt-1 break-all font-mono text-xs" : "mt-1 text-sm"}
      >
        {value}
      </dd>
    </div>
  );
}
