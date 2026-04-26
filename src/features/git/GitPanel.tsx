import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  GitBranch,
  GitCommit,
  PackageOpen,
  Package,
  RefreshCw,
  Zap,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  describeError,
  tauri,
  type GitBranch as GitBranchInfo,
  type GitChange,
  type GitChangeStatus,
  type GitStatus,
  type Site,
} from "@/lib/tauri";

interface Props {
  site: Site;
}

export function GitPanel({ site }: Props) {
  const queryClient = useQueryClient();
  const status = useQuery<GitStatus>({
    queryKey: ["git-status", site.id],
    queryFn: () => tauri.gitStatus(site.id),
    refetchInterval: false,
  });
  const [message, setMessage] = useState("");

  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["git-status", site.id] });

  const stage = useMutation({
    mutationFn: (paths: string[]) => tauri.gitStage(site.id, paths),
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });
  const unstage = useMutation({
    mutationFn: (paths: string[]) => tauri.gitUnstage(site.id, paths),
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });
  const commit = useMutation({
    mutationFn: () => tauri.gitCommit(site.id, message),
    onSuccess: () => {
      setMessage("");
      refresh();
    },
    onError: (e) => alert(describeError(e)),
  });
  const pull = useMutation({
    mutationFn: () => tauri.gitPull(site.id, "fastForward"),
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });
  const push = useMutation({
    mutationFn: () => tauri.gitPush(site.id),
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });
  const stash = useMutation({
    mutationFn: () => tauri.gitStashSave(site.id, "stash from Hugo Studio"),
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });
  const stashPop = useMutation({
    mutationFn: () => tauri.gitStashPop(site.id),
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });
  // "Force pull" = stash any working changes, then reset hard to upstream.
  // The stash preserves the user's edits even though the local commits
  // are discarded.
  const forcePull = useMutation({
    mutationFn: async () => {
      const dirty = (status.data?.changes.length ?? 0) > 0;
      if (dirty) {
        await tauri.gitStashSave(site.id, "auto-stash before force pull");
      }
      return tauri.gitPull(site.id, "forceReset");
    },
    onSuccess: (next) =>
      queryClient.setQueryData(["git-status", site.id], next),
    onError: (e) => alert(describeError(e)),
  });

  const grouped = useMemo(() => {
    const data = status.data;
    if (!data) return { staged: [], unstaged: [] };
    const staged: GitChange[] = [];
    const unstaged: GitChange[] = [];
    for (const c of data.changes) (c.staged ? staged : unstaged).push(c);
    staged.sort((a, b) => a.path.localeCompare(b.path));
    unstaged.sort((a, b) => a.path.localeCompare(b.path));
    return { staged, unstaged };
  }, [status.data]);

  if (status.isPending) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-muted-foreground">
        Reading git status…
      </p>
    );
  }
  if (status.isError) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-destructive">
        {describeError(status.error)}
      </p>
    );
  }

  const data = status.data!;

  if (!data.isRepo) {
    return (
      <div className="px-6 pb-10 pt-4">
        <div className="mx-auto max-w-md rounded-lg border border-dashed bg-muted/30 px-6 py-8 text-center text-sm">
          <GitBranch className="mx-auto size-6 text-muted-foreground" />
          <p className="mt-2 font-medium">Not a git repository</p>
          <p className="mt-1 text-xs text-muted-foreground">
            <code className="font-mono">{site.rootPath}</code> isn't tracked by
            git. Initialise from a terminal (
            <code className="font-mono">git init</code>), or use{" "}
            <span className="font-medium">Clone…</span> from the workspace
            screen to start from a remote.
          </p>
        </div>
      </div>
    );
  }

  const upstream = data.upstream ?? "(no upstream)";
  const canPush = !!data.upstream && data.ahead > 0;
  const canPull = !!data.upstream && data.behind > 0;

  return (
    // No `py-6` at the top — the panel sits flush against the
    // SiteShell's tab triggers. Bottom padding stays so the last
    // section doesn't kiss the edge of the scroll container.
    <div className="space-y-6 px-6 pb-6 pt-4">
      <header className="flex items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <GitBranch className="size-4 text-muted-foreground" />
            <BranchPicker
              site={site}
              currentBranch={data.branch}
              onSwitched={() =>
                queryClient.invalidateQueries({
                  queryKey: ["git-status", site.id],
                })
              }
            />
            <span className="text-xs text-muted-foreground">↔</span>
            <span className="font-mono text-xs text-muted-foreground">
              {upstream}
            </span>
          </div>
          <div className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
            <span
              title={`${data.ahead} commit(s) on the local branch not pushed`}
            >
              ↑ {data.ahead}
            </span>
            <span title={`${data.behind} commit(s) on the remote not pulled`}>
              ↓ {data.behind}
            </span>
            <span>· {data.changes.length} change(s)</span>
          </div>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-1">
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => pull.mutate()}
            disabled={!canPull || pull.isPending}
            title="Fast-forward only"
          >
            <ArrowDownToLine className="size-4" />
            {pull.isPending ? "Pulling…" : "Pull"}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => push.mutate()}
            disabled={!canPush || push.isPending}
          >
            <ArrowUpFromLine className="size-4" />
            {push.isPending ? "Pushing…" : "Push"}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => stash.mutate()}
            disabled={data.changes.length === 0 || stash.isPending}
            title="git stash all changes"
          >
            <Package className="size-4" />
            {stash.isPending ? "Stashing…" : "Stash"}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => stashPop.mutate()}
            disabled={stashPop.isPending}
            title="git stash pop (apply most recent stash)"
          >
            <PackageOpen className="size-4" />
            {stashPop.isPending ? "Popping…" : "Pop stash"}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="destructive"
            onClick={() => {
              if (
                confirm(
                  "Force pull will discard local commits and reset to upstream. " +
                    "Working-tree changes will be auto-stashed first. Continue?",
                )
              ) {
                forcePull.mutate();
              }
            }}
            disabled={!data.upstream || forcePull.isPending}
            title="Stash + reset --hard to upstream"
          >
            <Zap className="size-4" />
            {forcePull.isPending ? "Forcing…" : "Force pull"}
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={refresh}
            aria-label="Refresh git status"
            title="Refresh"
          >
            <RefreshCw className="size-4" />
          </Button>
        </div>
      </header>

      <ChangeList
        title="Staged"
        items={grouped.staged}
        empty="Nothing staged for the next commit."
        actionLabel="Unstage"
        onAct={(paths) => unstage.mutate(paths)}
        busy={unstage.isPending}
      />

      <ChangeList
        title="Working tree"
        items={grouped.unstaged}
        empty="No working-tree changes."
        actionLabel="Stage"
        onAct={(paths) => stage.mutate(paths)}
        busy={stage.isPending}
      />

      <section>
        <h3 className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          <GitCommit className="size-3.5" />
          Commit
        </h3>
        <Input
          type="text"
          placeholder="Commit message"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
        />
        <div className="mt-2 flex items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>
            {grouped.staged.length === 0
              ? "Stage some changes first."
              : `${grouped.staged.length} change(s) staged.`}
          </span>
          <Button
            type="button"
            size="sm"
            disabled={
              !message.trim() || grouped.staged.length === 0 || commit.isPending
            }
            onClick={() => commit.mutate()}
          >
            <GitCommit className="size-4" />
            {commit.isPending ? "Committing…" : "Commit"}
          </Button>
        </div>
      </section>
    </div>
  );
}

function BranchPicker({
  site,
  currentBranch,
  onSwitched,
}: {
  site: Site;
  currentBranch: string | null;
  onSwitched: () => void;
}) {
  const queryClient = useQueryClient();
  const branches = useQuery<GitBranchInfo[]>({
    queryKey: ["git-branches", site.id],
    queryFn: () => tauri.gitBranches(site.id),
  });

  const checkout = useMutation({
    mutationFn: (branch: string) => tauri.gitCheckout(site.id, branch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["git-branches", site.id] });
      queryClient.invalidateQueries({ queryKey: ["content", site.id] });
      onSwitched();
    },
    onError: (e) => alert(describeError(e)),
  });

  const createBranch = useMutation({
    mutationFn: (name: string) => tauri.gitBranchCreate(site.id, name, true),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["git-branches", site.id] });
      onSwitched();
    },
    onError: (e) => alert(describeError(e)),
  });

  const SENTINEL_NEW = "__new__";

  function handleChange(value: string) {
    if (value === SENTINEL_NEW) {
      const name = prompt(
        "New branch name (will check out from current HEAD):",
      );
      if (!name || !name.trim()) return;
      createBranch.mutate(name.trim());
      return;
    }
    if (value && value !== currentBranch) {
      checkout.mutate(value);
    }
  }

  const list = branches.data ?? [];

  return (
    <select
      value={currentBranch ?? ""}
      onChange={(e) => handleChange(e.target.value)}
      disabled={
        branches.isPending || checkout.isPending || createBranch.isPending
      }
      className="rounded-md border bg-background px-2 py-1 text-base font-semibold"
      title="Switch branch"
    >
      {currentBranch == null && <option value="">(detached HEAD)</option>}
      {list.map((b) => (
        <option key={b.name} value={b.name}>
          {b.name}
          {b.upstream ? ` ↔ ${b.upstream}` : ""}
        </option>
      ))}
      <option disabled>──────────</option>
      <option value={SENTINEL_NEW}>+ New branch from HEAD…</option>
    </select>
  );
}

function ChangeList({
  title,
  items,
  empty,
  actionLabel,
  onAct,
  busy,
}: {
  title: string;
  items: GitChange[];
  empty: string;
  actionLabel: string;
  onAct: (paths: string[]) => void;
  busy: boolean;
}) {
  return (
    <section>
      <header className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          {title} ({items.length})
        </h3>
        {items.length > 0 && (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => onAct(items.map((i) => i.path))}
            disabled={busy}
          >
            {actionLabel} all
          </Button>
        )}
      </header>
      {items.length === 0 ? (
        <p className="text-xs text-muted-foreground">{empty}</p>
      ) : (
        <ul className="space-y-0.5 rounded-md border bg-muted/20 p-1">
          {items.map((c) => (
            <li
              key={`${c.path}-${c.staged ? "s" : "w"}`}
              className="flex items-center justify-between gap-2 rounded px-2 py-1 text-xs hover:bg-accent"
            >
              <button
                type="button"
                onClick={() => onAct([c.path])}
                disabled={busy}
                className="flex flex-1 items-center gap-2 truncate text-left"
              >
                <StatusBadge status={c.status} />
                <span className="truncate font-mono">{c.path}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function StatusBadge({ status }: { status: GitChangeStatus }) {
  const meta = badgeMeta(status);
  return (
    <span
      className={cn(
        "inline-flex w-12 shrink-0 justify-center rounded border px-1 py-0 text-[9px] font-semibold uppercase tracking-wider",
        meta.tone,
      )}
      title={status}
    >
      {meta.label}
    </span>
  );
}

function badgeMeta(status: GitChangeStatus): { label: string; tone: string } {
  switch (status) {
    case "new":
      return {
        label: "New",
        tone: "border-emerald-300 bg-emerald-50 text-emerald-900",
      };
    case "modified":
      return {
        label: "Mod",
        tone: "border-amber-300 bg-amber-50 text-amber-900",
      };
    case "deleted":
      return {
        label: "Del",
        tone: "border-destructive/40 bg-destructive/10 text-destructive",
      };
    case "renamed":
      return { label: "Ren", tone: "border-sky-300 bg-sky-50 text-sky-900" };
    case "untracked":
      return {
        label: "?",
        tone: "border-muted-foreground/30 bg-muted text-muted-foreground",
      };
    case "conflicted":
      return {
        label: "!",
        tone: "border-destructive bg-destructive/20 text-destructive",
      };
    case "ignored":
      return {
        label: "Ign",
        tone: "border-muted-foreground/30 bg-muted text-muted-foreground",
      };
    case "typeChange":
      return {
        label: "Typ",
        tone: "border-purple-300 bg-purple-50 text-purple-900",
      };
    default:
      return {
        label: "?",
        tone: "border-muted-foreground/30 bg-muted text-muted-foreground",
      };
  }
}
