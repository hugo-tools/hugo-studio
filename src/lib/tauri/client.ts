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
  ArchetypeContent,
  ArchetypeKind,
  CreateOptions,
  CreatedContent,
  DataFile,
  DataFileContent,
  DetectionInfo,
  ThemeFileContent,
  ThemeFilesIndex,
  GitBranch,
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
  archetypeRead: (siteId: SiteId, name: string): Promise<ArchetypeContent> =>
    unwrap(commands.archetypeRead(siteId, name)),
  archetypeWrite: (
    siteId: SiteId,
    name: string,
    text: string,
  ): Promise<ArchetypeContent> =>
    unwrap(commands.archetypeWrite(siteId, name, text)),
  archetypeCreate: (
    siteId: SiteId,
    name: string,
    kind: ArchetypeKind,
    text: string | null = null,
  ): Promise<ArchetypeContent> =>
    unwrap(commands.archetypeCreate(siteId, name, kind, text)),
  archetypeDelete: (siteId: SiteId, name: string): Promise<null> =>
    unwrap(commands.archetypeDelete(siteId, name)),
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
  themeFilesList: (siteId: SiteId): Promise<ThemeFilesIndex> =>
    unwrap(commands.themeFilesList(siteId)),
  themeFileRead: (siteId: SiteId, relPath: string): Promise<ThemeFileContent> =>
    unwrap(commands.themeFileRead(siteId, relPath)),
  themeFileWrite: (
    siteId: SiteId,
    relPath: string,
    text: string,
  ): Promise<ThemeFileContent> =>
    unwrap(commands.themeFileWrite(siteId, relPath, text)),
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
  assetListStatic: (siteId: SiteId): Promise<AssetRef[]> =>
    unwrap(commands.assetListStatic(siteId)),
  assetListAssets: (siteId: SiteId): Promise<AssetRef[]> =>
    unwrap(commands.assetListAssets(siteId)),
  assetDelete: (siteId: SiteId, assetId: string): Promise<null> =>
    unwrap(commands.assetDelete(siteId, assetId)),
  dataList: (siteId: SiteId): Promise<DataFile[]> =>
    unwrap(commands.dataList(siteId)),
  dataRead: (siteId: SiteId, relPath: string): Promise<DataFileContent> =>
    unwrap(commands.dataRead(siteId, relPath)),
  dataWrite: (
    siteId: SiteId,
    relPath: string,
    text: string,
  ): Promise<DataFileContent> =>
    unwrap(commands.dataWrite(siteId, relPath, text)),
  dataCreate: (siteId: SiteId, relPath: string): Promise<DataFile> =>
    unwrap(commands.dataCreate(siteId, relPath)),
  dataImport: (siteId: SiteId, source: string): Promise<DataFile> =>
    unwrap(commands.dataImport(siteId, source)),
  dataDelete: (siteId: SiteId, relPath: string): Promise<null> =>
    unwrap(commands.dataDelete(siteId, relPath)),
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
  gitBranches: (siteId: SiteId): Promise<GitBranch[]> =>
    unwrap(commands.gitBranches(siteId)),
  gitCheckout: (siteId: SiteId, branch: string): Promise<GitStatus> =>
    unwrap(commands.gitCheckout(siteId, branch)),
  gitBranchCreate: (
    siteId: SiteId,
    name: string,
    checkoutAfter: boolean,
  ): Promise<GitStatus> =>
    unwrap(commands.gitBranchCreate(siteId, name, checkoutAfter)),
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
