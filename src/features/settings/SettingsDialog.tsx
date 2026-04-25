import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  CheckCircle2,
  FolderOpen,
  Monitor,
  Moon,
  RotateCcw,
  Sun,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { describeError, tauri, type AppSettings } from "@/lib/tauri";
import { useThemeStore, type ThemeMode } from "@/store/theme";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsDialog({ open, onOpenChange }: Props) {
  const queryClient = useQueryClient();
  const themeMode = useThemeStore((s) => s.mode);
  const setThemeMode = useThemeStore((s) => s.setMode);

  const settings = useQuery<AppSettings>({
    queryKey: ["app-settings"],
    queryFn: () => tauri.appSettingsGet(),
    enabled: open,
  });
  const resolved = useQuery<string | null>({
    queryKey: ["resolved-hugo"],
    queryFn: () => tauri.appSettingsResolveHugo(),
    enabled: open,
    refetchOnWindowFocus: false,
  });

  const [hugoPath, setHugoPath] = useState("");

  useEffect(() => {
    if (settings.data) setHugoPath(settings.data.hugoPath ?? "");
  }, [settings.data]);

  const save = useMutation({
    mutationFn: (next: AppSettings) => tauri.appSettingsSave(next),
    onSuccess: (s) => {
      queryClient.setQueryData(["app-settings"], s);
      queryClient.invalidateQueries({ queryKey: ["resolved-hugo"] });
    },
    onError: (e) => alert(describeError(e)),
  });

  const pickHugo = useMutation({
    mutationFn: async () => {
      const picked = await openFile({
        directory: false,
        multiple: false,
        title: "Select the Hugo binary",
      });
      if (typeof picked === "string") setHugoPath(picked);
    },
  });

  const dirtyHugo =
    (settings.data?.hugoPath ?? "") !== (hugoPath || (null as never));

  function handleSave() {
    save.mutate({
      hugoPath: hugoPath.trim() === "" ? null : hugoPath.trim(),
    });
  }

  function clearHugo() {
    setHugoPath("");
    save.mutate({ hugoPath: null });
  }

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="max-w-lg">
        <AlertDialogHeader>
          <AlertDialogTitle>Settings</AlertDialogTitle>
          <AlertDialogDescription>
            Preferences are stored next to the workspace file in your OS app
            data directory.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-6 py-2">
          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Theme
            </h3>
            <div className="flex gap-1">
              {(["light", "dark", "system"] as ThemeMode[]).map((mode) => {
                const Icon =
                  mode === "dark" ? Moon : mode === "light" ? Sun : Monitor;
                return (
                  <Button
                    key={mode}
                    type="button"
                    variant={themeMode === mode ? "default" : "outline"}
                    size="sm"
                    onClick={() => setThemeMode(mode)}
                    className="flex-1"
                  >
                    <Icon className="size-4" />
                    {mode[0].toUpperCase() + mode.slice(1)}
                  </Button>
                );
              })}
            </div>
          </section>

          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Hugo binary
            </h3>
            <div className="flex gap-2">
              <Input
                type="text"
                value={hugoPath}
                onChange={(e) => setHugoPath(e.target.value)}
                placeholder="(leave empty to use PATH)"
                className="font-mono text-xs"
              />
              <Button
                type="button"
                size="icon"
                variant="outline"
                onClick={() => pickHugo.mutate()}
                disabled={pickHugo.isPending}
                aria-label="Pick Hugo binary"
                title="Browse"
              >
                <FolderOpen className="size-4" />
              </Button>
              <Button
                type="button"
                size="icon"
                variant="ghost"
                onClick={clearHugo}
                disabled={hugoPath === "" || save.isPending}
                aria-label="Clear and use PATH"
                title="Clear (use PATH)"
              >
                <RotateCcw className="size-4" />
              </Button>
            </div>
            <ResolvedHint
              loading={resolved.isPending || save.isPending}
              path={resolved.data ?? null}
            />
            {dirtyHugo && (
              <Button
                type="button"
                size="sm"
                className="mt-2"
                onClick={handleSave}
                disabled={save.isPending}
              >
                {save.isPending ? "Saving…" : "Save Hugo path"}
              </Button>
            )}
          </section>
        </div>

        <AlertDialogFooter>
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
          >
            <X className="size-4" />
            Close
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function ResolvedHint({
  loading,
  path,
}: {
  loading: boolean;
  path: string | null;
}) {
  if (loading) {
    return <p className="mt-2 text-xs text-muted-foreground">Resolving…</p>;
  }
  if (path) {
    return (
      <p
        className={cn(
          "mt-2 flex items-center gap-1.5 text-xs",
          "text-emerald-700 dark:text-emerald-400",
        )}
        title={path}
      >
        <CheckCircle2 className="size-3.5" />
        <span className="truncate font-mono">Will use: {path}</span>
      </p>
    );
  }
  return (
    <p className="mt-2 flex items-center gap-1.5 text-xs text-amber-700 dark:text-amber-400">
      <AlertCircle className="size-3.5" />
      <span>
        Hugo not found. Install it and add to PATH, or pick the binary above.
      </span>
    </p>
  );
}
