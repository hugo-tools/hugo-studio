import { create } from "zustand";

import type { Site } from "@/lib/tauri";

export interface EditorSelection {
  /** Absolute path of the file on disk — passed to content_get / save. */
  path: string;
  /** Stable id from ContentSummary (path-rel, no language suffix). */
  id: string;
  language: string;
  /** Cached title for header display before content_get arrives. */
  title?: string | null;
}

interface WorkspaceState {
  activeSite: Site | null;
  setActiveSite: (site: Site | null) => void;
  /** When set, SiteShell shows the editor instead of the settings panel. */
  selection: EditorSelection | null;
  selectContent: (sel: EditorSelection | null) => void;
}

/**
 * Client-side mirror of the active-site decision so the UI can render the
 * right surface (workspace list vs. site shell) without a roundtrip.
 * The source of truth still lives in the backend (`workspace.json`); this
 * store is reset by `<App>` from the backend on mount.
 */
export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  activeSite: null,
  setActiveSite: (site) =>
    set({
      activeSite: site,
      // Switching site clears any in-progress editor selection.
      selection: null,
    }),
  selection: null,
  selectContent: (sel) => set({ selection: sel }),
}));
