import { create } from "zustand";

import type { Site } from "@/lib/tauri";

interface WorkspaceState {
  activeSite: Site | null;
  setActiveSite: (site: Site | null) => void;
}

/**
 * Client-side mirror of the active-site decision so the UI can render the
 * right surface (workspace list vs. site shell) without a roundtrip.
 * The source of truth still lives in the backend (`workspace.json`); this
 * store is reset by `<App>` from the backend on mount.
 */
export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  activeSite: null,
  setActiveSite: (site) => set({ activeSite: site }),
}));
