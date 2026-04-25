import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { FolderPlus, GitBranch } from "lucide-react";

import { Button } from "@/components/ui/button";
import { tauri, type SiteRef } from "@/lib/tauri";
import { ThemeToggle } from "@/features/theme-mode/ThemeToggle";
import { CloneDialog } from "@/features/git/CloneDialog";
import { AddSiteButton } from "./AddSiteButton";
import { SiteCard } from "./SiteCard";

export function WorkspaceScreen() {
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [cloneOpen, setCloneOpen] = useState(false);

  const sites = useQuery<SiteRef[]>({
    queryKey: ["sites"],
    queryFn: () => tauri.workspaceListSites(),
  });

  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex items-center justify-between border-b px-6 py-4">
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Hugo Studio</h1>
          <p className="text-xs text-muted-foreground">
            Workspace · {sites.data?.length ?? 0} site
            {sites.data?.length === 1 ? "" : "s"}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <ThemeToggle />
          <Button
            variant="outline"
            onClick={() => setCloneOpen(true)}
            type="button"
          >
            <GitBranch className="size-4" />
            Clone…
          </Button>
          <AddSiteButton onError={setErrorMsg} />
        </div>
      </header>

      <main className="flex-1 px-6 py-8">
        {errorMsg && (
          <div
            role="alert"
            className="mb-6 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-2 text-sm text-destructive"
          >
            {errorMsg}
          </div>
        )}

        {sites.isPending && (
          <p className="text-sm text-muted-foreground">Loading workspace…</p>
        )}

        {sites.isError && (
          <p className="text-sm text-destructive">
            Failed to load workspace: {String(sites.error)}
          </p>
        )}

        {sites.data && sites.data.length === 0 && (
          <EmptyState
            onError={setErrorMsg}
            onCloneClick={() => setCloneOpen(true)}
          />
        )}

        {sites.data && sites.data.length > 0 && (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {sites.data.map((site) => (
              <SiteCard key={site.id} site={site} onError={setErrorMsg} />
            ))}
          </div>
        )}
      </main>

      <CloneDialog open={cloneOpen} onOpenChange={setCloneOpen} />
    </div>
  );
}

function EmptyState({
  onError,
  onCloneClick,
}: {
  onError: (msg: string) => void;
  onCloneClick: () => void;
}) {
  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-4 rounded-lg border border-dashed py-16 text-center">
      <FolderPlus className="size-10 text-muted-foreground" />
      <div>
        <h2 className="text-lg font-medium">No sites yet</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Point Hugo Studio at the root folder of any Hugo site, or clone one
          straight from a remote.
        </p>
      </div>
      <div className="flex items-center gap-2">
        <Button variant="outline" onClick={onCloneClick}>
          <GitBranch className="size-4" />
          Clone…
        </Button>
        <AddSiteButton onError={onError} />
      </div>
    </div>
  );
}
