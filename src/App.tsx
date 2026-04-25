import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";

import { tauri } from "@/lib/tauri";
import { useWorkspaceStore } from "@/store/workspace";
import { WorkspaceScreen } from "@/features/workspace/WorkspaceScreen";
import { SiteShell } from "@/features/site/SiteShell";

/**
 * On mount we ask the backend for its persisted active site (if any) and
 * reopen it so a refresh / restart lands the user back where they were.
 */
export function App() {
  const activeSite = useWorkspaceStore((s) => s.activeSite);
  const setActiveSite = useWorkspaceStore((s) => s.setActiveSite);

  const restored = useQuery({
    queryKey: ["active-site-bootstrap"],
    queryFn: async () => {
      const id = await tauri.workspaceActiveSiteId();
      if (!id) return null;
      return tauri.workspaceSetActive(id);
    },
    staleTime: Infinity,
    gcTime: Infinity,
  });

  useEffect(() => {
    if (restored.data && !activeSite) setActiveSite(restored.data);
  }, [restored.data, activeSite, setActiveSite]);

  if (restored.isPending) {
    return (
      <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">
        Loading workspace…
      </div>
    );
  }

  return activeSite ? <SiteShell site={activeSite} /> : <WorkspaceScreen />;
}
