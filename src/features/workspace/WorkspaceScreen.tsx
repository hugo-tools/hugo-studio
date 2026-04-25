import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { FolderPlus } from "lucide-react";

import { tauri, type SiteRef } from "@/lib/tauri";
import { AddSiteButton } from "./AddSiteButton";
import { SiteCard } from "./SiteCard";

export function WorkspaceScreen() {
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

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
        <AddSiteButton onError={setErrorMsg} />
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
          <EmptyState onError={setErrorMsg} />
        )}

        {sites.data && sites.data.length > 0 && (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {sites.data.map((site) => (
              <SiteCard key={site.id} site={site} onError={setErrorMsg} />
            ))}
          </div>
        )}
      </main>
    </div>
  );
}

function EmptyState({ onError }: { onError: (msg: string) => void }) {
  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-4 rounded-lg border border-dashed py-16 text-center">
      <FolderPlus className="size-10 text-muted-foreground" />
      <div>
        <h2 className="text-lg font-medium">No sites yet</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Point Hugo Studio at the root folder of any Hugo site to start
          editing.
        </p>
      </div>
      <AddSiteButton onError={onError} />
    </div>
  );
}
