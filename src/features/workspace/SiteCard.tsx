import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { FolderOpen, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { tauri, describeError, type SiteRef } from "@/lib/tauri";
import { useWorkspaceStore } from "@/store/workspace";

interface Props {
  site: SiteRef;
  onError?: (message: string) => void;
}

function formatRelative(iso: string): string {
  const then = new Date(iso).getTime();
  const now = Date.now();
  const sec = Math.max(0, Math.floor((now - then) / 1000));
  if (sec < 60) return "just now";
  if (sec < 3600) return `${Math.floor(sec / 60)} min ago`;
  if (sec < 86_400) return `${Math.floor(sec / 3600)} h ago`;
  return new Date(iso).toLocaleDateString();
}

export function SiteCard({ site, onError }: Props) {
  const queryClient = useQueryClient();
  const setActiveSite = useWorkspaceStore((s) => s.setActiveSite);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const openSite = useMutation({
    mutationFn: () => tauri.workspaceSetActive(site.id),
    onSuccess: (opened) => {
      setActiveSite(opened);
      queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
    onError: (err) => onError?.(describeError(err)),
  });

  const removeSite = useMutation({
    mutationFn: () => tauri.workspaceRemoveSite(site.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sites"] });
      setConfirmOpen(false);
    },
    onError: (err) => onError?.(describeError(err)),
  });

  return (
    <Card className="flex flex-col">
      <CardHeader className="pb-3">
        <CardTitle className="truncate" title={site.name}>
          {site.name}
        </CardTitle>
        <CardDescription className="truncate text-xs" title={site.rootPath}>
          {site.rootPath}
        </CardDescription>
      </CardHeader>
      <CardContent className="flex-1 pb-3 text-xs text-muted-foreground">
        Last opened {formatRelative(site.lastOpened)}
      </CardContent>
      <CardFooter className="justify-between gap-2 pt-0">
        <Button
          size="sm"
          onClick={() => openSite.mutate()}
          disabled={openSite.isPending}
        >
          <FolderOpen className="size-4" />
          {openSite.isPending ? "Opening…" : "Open"}
        </Button>
        <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
          <AlertDialogTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              aria-label={`Remove ${site.name}`}
            >
              <Trash2 className="size-4" />
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Remove site from workspace?</AlertDialogTitle>
              <AlertDialogDescription>
                This only removes <strong>{site.name}</strong> from the Hugo
                Studio workspace list. The folder on disk is left untouched.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={removeSite.isPending}>
                Cancel
              </AlertDialogCancel>
              <AlertDialogAction
                disabled={removeSite.isPending}
                onClick={(e) => {
                  e.preventDefault();
                  removeSite.mutate();
                }}
              >
                {removeSite.isPending ? "Removing…" : "Remove"}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardFooter>
    </Card>
  );
}
