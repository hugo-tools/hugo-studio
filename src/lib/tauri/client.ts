// Thin wrapper that turns the tauri-specta `Result<T, AppError>` envelope
// into a normal Promise<T> that rejects with the typed error.
// Lets us call commands directly from TanStack Query without a per-callsite
// `if (status === "ok") …` dance.

import { commands, type Result } from "./bindings";
import type {
  AppError,
  Site,
  SiteId,
  SiteRef,
  DetectionInfo,
} from "./bindings";

async function unwrap<T>(p: Promise<Result<T, AppError>>): Promise<T> {
  const res = await p;
  if (res.status === "ok") return res.data;
  throw res.error;
}

export const tauri = {
  healthCheck: () => commands.healthCheck(),
  siteDetect: (path: string): Promise<DetectionInfo> =>
    unwrap(commands.siteDetect(path)),
  workspaceListSites: (): Promise<SiteRef[]> =>
    unwrap(commands.workspaceListSites()),
  workspaceActiveSiteId: (): Promise<SiteId | null> =>
    unwrap(commands.workspaceActiveSiteId()),
  workspaceAddSite: (path: string, name: string | null): Promise<SiteRef> =>
    unwrap(commands.workspaceAddSite(path, name)),
  workspaceRemoveSite: (id: SiteId): Promise<null> =>
    unwrap(commands.workspaceRemoveSite(id)),
  workspaceRenameSite: (id: SiteId, name: string): Promise<SiteRef> =>
    unwrap(commands.workspaceRenameSite(id, name)),
  workspaceSetActive: (id: SiteId): Promise<Site> =>
    unwrap(commands.workspaceSetActive(id)),
  workspaceClearActive: (): Promise<null> =>
    unwrap(commands.workspaceClearActive()),
};

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    typeof (value as { kind: unknown }).kind === "string"
  );
}

export function describeError(value: unknown): string {
  if (isAppError(value)) return value.message;
  if (value instanceof Error) return value.message;
  return String(value);
}
