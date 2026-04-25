// Thin wrapper that turns the tauri-specta `Result<T, AppError>` envelope
// into a normal Promise<T> that rejects with the typed error.
// Lets us call commands directly from TanStack Query without a per-callsite
// `if (status === "ok") …` dance.

import { commands, type Result } from "./bindings";
import type {
  AppError,
  AppSettings,
  Archetype,
  AssetContext,
  AssetRef,
  CloneOptions,
  CloneResult,
  CommitResult,
  ContentEditPayload,
  ContentScanResult,
  CreateOptions,
  CreatedContent,
  DetectionInfo,
  GitStatus,
  JsonValue,
  LoadedConfig,
  PreviewHandle,
  PreviewStatus,
  Site,
  SiteId,
  SiteRef,
  ThemeInfo,
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
  configGet: (siteId: SiteId): Promise<LoadedConfig> =>
    unwrap(commands.configGet(siteId)),
  configSave: (siteId: SiteId, merged: JsonValue): Promise<LoadedConfig> =>
    unwrap(commands.configSave(siteId, merged)),
  contentList: (siteId: SiteId): Promise<ContentScanResult> =>
    unwrap(commands.contentList(siteId)),
  contentArchetypes: (siteId: SiteId): Promise<Archetype[]> =>
    unwrap(commands.contentArchetypes(siteId)),
  contentCreate: (
    siteId: SiteId,
    options: CreateOptions,
  ): Promise<CreatedContent> => unwrap(commands.contentCreate(siteId, options)),
  contentGet: (siteId: SiteId, path: string): Promise<ContentEditPayload> =>
    unwrap(commands.contentGet(siteId, path)),
  contentSave: (
    siteId: SiteId,
    path: string,
    frontMatter: JsonValue,
    body: string,
  ): Promise<ContentEditPayload> =>
    unwrap(commands.contentSave(siteId, path, frontMatter, body)),
  themeGet: (siteId: SiteId): Promise<ThemeInfo> =>
    unwrap(commands.themeGet(siteId)),
  themeSaveParams: (siteId: SiteId, params: JsonValue): Promise<ThemeInfo> =>
    unwrap(commands.themeSaveParams(siteId, params)),
  previewStart: (siteId: SiteId): Promise<PreviewHandle> =>
    unwrap(commands.previewStart(siteId)),
  previewStop: (): Promise<null> => unwrap(commands.previewStop()),
  previewStatus: (): Promise<PreviewStatus> => unwrap(commands.previewStatus()),
  assetImport: (
    siteId: SiteId,
    source: string,
    targetContext: AssetContext,
  ): Promise<AssetRef> =>
    unwrap(commands.assetImport(siteId, source, targetContext)),
  assetList: (siteId: SiteId, contentId: string | null): Promise<AssetRef[]> =>
    unwrap(commands.assetList(siteId, contentId)),
  assetDelete: (siteId: SiteId, assetId: string): Promise<null> =>
    unwrap(commands.assetDelete(siteId, assetId)),
  gitStatus: (siteId: SiteId): Promise<GitStatus> =>
    unwrap(commands.gitStatus(siteId)),
  gitClone: (opts: CloneOptions): Promise<CloneResult> =>
    unwrap(commands.gitClone(opts)),
  gitStage: (siteId: SiteId, paths: string[]): Promise<GitStatus> =>
    unwrap(commands.gitStage(siteId, paths)),
  gitUnstage: (siteId: SiteId, paths: string[]): Promise<GitStatus> =>
    unwrap(commands.gitUnstage(siteId, paths)),
  gitCommit: (siteId: SiteId, message: string): Promise<CommitResult> =>
    unwrap(commands.gitCommit(siteId, message)),
  gitPull: (
    siteId: SiteId,
    strategy: "fastForward" | "forceReset" = "fastForward",
  ): Promise<GitStatus> => unwrap(commands.gitPull(siteId, strategy)),
  gitPush: (siteId: SiteId): Promise<GitStatus> =>
    unwrap(commands.gitPush(siteId)),
  gitStashSave: (siteId: SiteId, message: string): Promise<GitStatus> =>
    unwrap(commands.gitStashSave(siteId, message)),
  gitStashPop: (siteId: SiteId): Promise<GitStatus> =>
    unwrap(commands.gitStashPop(siteId)),
  appSettingsGet: (): Promise<AppSettings> => unwrap(commands.appSettingsGet()),
  appSettingsSave: (next: AppSettings): Promise<AppSettings> =>
    unwrap(commands.appSettingsSave(next)),
  appSettingsResolveHugo: (): Promise<string | null> =>
    unwrap(commands.appSettingsResolveHugo()),
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
  if (isAppError(value)) {
    return "message" in value ? value.message : value.kind;
  }
  if (value instanceof Error) return value.message;
  return String(value);
}
