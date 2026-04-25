import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open as openFolder } from "@tauri-apps/plugin-dialog";
import { GitBranch, FolderOpen } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import { describeError, tauri } from "@/lib/tauri";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Derive a sane folder name from a git URL. */
function repoNameFromUrl(url: string): string {
  let s = url.trim();
  if (s.endsWith(".git")) s = s.slice(0, -4);
  s = s.replace(/[\\/]+$/, "");
  const idx = Math.max(s.lastIndexOf("/"), s.lastIndexOf(":"));
  return idx >= 0 ? s.slice(idx + 1) : s;
}

function joinPath(parent: string, child: string): string {
  if (!parent) return child;
  const sep = parent.includes("\\") ? "\\" : "/";
  const trimmed = parent.replace(/[\\/]+$/, "");
  return `${trimmed}${sep}${child}`;
}

export function CloneDialog({ open, onOpenChange }: Props) {
  const queryClient = useQueryClient();
  const [url, setUrl] = useState("");
  const [parent, setParent] = useState("");
  const [branch, setBranch] = useState("");

  const repoName = repoNameFromUrl(url);
  const computedDest = parent && repoName ? joinPath(parent, repoName) : "";

  const pickFolder = useMutation({
    mutationFn: async () => {
      const picked = await openFolder({
        directory: true,
        multiple: false,
        title: "Pick the parent folder for the new clone",
      });
      if (typeof picked === "string") setParent(picked);
    },
  });

  const clone = useMutation({
    mutationFn: async () => {
      if (!url.trim()) throw new Error("repository URL required");
      if (!computedDest) throw new Error("destination folder required");
      const result = await tauri.gitClone({
        url: url.trim(),
        dest: computedDest,
        branch: branch.trim() || null,
      });
      // Register the cloned site in the workspace. Will throw if the
      // repo isn't actually a Hugo site — that's the right behavior:
      // the user can find it on disk and pick a different repo.
      try {
        await tauri.workspaceAddSite(result.dest, repoName || null);
      } catch (e) {
        // Re-throw with a more helpful message.
        throw new Error(
          `Cloned to ${result.dest} but the directory is not a Hugo site: ${describeError(e)}`,
        );
      }
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sites"] });
      onOpenChange(false);
      setUrl("");
      setParent("");
      setBranch("");
    },
  });

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Clone a Hugo site repository</AlertDialogTitle>
          <AlertDialogDescription>
            SSH URLs use your <code className="font-mono">ssh-agent</code>;
            HTTPS uses the system git credential helper.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-3 py-2">
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium">Repository URL</span>
            <Input
              type="text"
              placeholder="git@github.com:user/site.git"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              autoFocus
            />
          </label>

          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium">Parent folder</span>
            <div className="flex items-center gap-2">
              <Input
                type="text"
                placeholder="Pick where the clone should live"
                value={parent}
                onChange={(e) => setParent(e.target.value)}
              />
              <Button
                type="button"
                size="icon"
                variant="outline"
                onClick={() => pickFolder.mutate()}
                disabled={pickFolder.isPending}
                aria-label="Browse for parent folder"
              >
                <FolderOpen className="size-4" />
              </Button>
            </div>
            {computedDest && (
              <span className="font-mono text-xs text-muted-foreground">
                → {computedDest}
              </span>
            )}
          </label>

          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium">
              Branch <span className="text-muted-foreground">(optional)</span>
            </span>
            <Input
              type="text"
              placeholder="main"
              value={branch}
              onChange={(e) => setBranch(e.target.value)}
            />
          </label>

          {clone.isError && (
            <p className="text-xs text-destructive">
              {describeError(clone.error)}
            </p>
          )}
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={clone.isPending}>
            Cancel
          </AlertDialogCancel>
          <Button
            type="button"
            onClick={() => clone.mutate()}
            disabled={clone.isPending || !url.trim() || !computedDest}
          >
            <GitBranch className="size-4" />
            {clone.isPending ? "Cloning…" : "Clone"}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
