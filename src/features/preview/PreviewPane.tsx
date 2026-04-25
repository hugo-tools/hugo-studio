import { useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Play, RotateCw, Square, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { describeError, tauri, type Site } from "@/lib/tauri";
import { usePreviewStore } from "@/store/preview";
import { PreviewConsole } from "./PreviewConsole";

interface Props {
  site: Site;
  onClose: () => void;
}

interface ReadyPayload {
  url: string;
  port: number;
}

interface LogPayload {
  stream: "stdout" | "stderr";
  line: string;
}

interface ErrorPayload {
  message: string;
  tail: string[];
}

interface ExitedPayload {
  reason: string;
  code: number | null;
}

export function PreviewPane({ site, onClose }: Props) {
  const queryClient = useQueryClient();
  const lifecycle = usePreviewStore((s) => s.lifecycle);
  const setLifecycle = usePreviewStore((s) => s.setLifecycle);
  const pushLog = usePreviewStore((s) => s.pushLog);
  const resetLogs = usePreviewStore((s) => s.resetLogs);
  const consoleOpen = usePreviewStore((s) => s.consoleOpen);
  const setConsoleOpen = usePreviewStore((s) => s.setConsoleOpen);

  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [iframeKey, setIframeKey] = useState(0);

  // Wire the four backend events to the lifecycle / log store.
  useEffect(() => {
    let cancelled = false;
    const unlisteners: UnlistenFn[] = [];
    async function attach() {
      const ready = await listen<ReadyPayload>("preview:ready", (e) => {
        setLifecycle({
          status: "running",
          siteId: site.id,
          url: e.payload.url,
          port: e.payload.port,
          hugoPath: "",
        });
      });
      const log = await listen<LogPayload>("preview:log", (e) => {
        pushLog({ ...e.payload, at: Date.now() });
      });
      const err = await listen<ErrorPayload>("preview:error", (e) => {
        setLifecycle({
          status: "error",
          siteId: site.id,
          message: e.payload.message,
          tail: e.payload.tail,
        });
      });
      const exited = await listen<ExitedPayload>("preview:exited", () => {
        setLifecycle((cur) =>
          cur.status === "error" ? cur : { status: "stopped", siteId: site.id },
        );
      });
      if (cancelled) {
        ready();
        log();
        err();
        exited();
        return;
      }
      unlisteners.push(ready, log, err, exited);
    }
    attach();
    return () => {
      cancelled = true;
      for (const u of unlisteners) u();
    };
    // We intentionally only re-arm listeners when the active site changes —
    // the global preview store is shared across mounts.
  }, [site.id, setLifecycle, pushLog]);

  // Switching site clears the lifecycle so we don't show stale state.
  useEffect(() => {
    return () => {
      setLifecycle({ status: "idle" });
      resetLogs();
    };
  }, [site.id, setLifecycle, resetLogs]);

  const start = useMutation({
    mutationFn: async () => {
      resetLogs();
      setLifecycle({ status: "starting", siteId: site.id });
      return tauri.previewStart(site.id);
    },
    onSuccess: (handle) => {
      // Backend will also emit preview:ready; we set hugoPath immediately so
      // the header can show it without waiting.
      setLifecycle((cur) =>
        cur.status === "running"
          ? { ...cur, hugoPath: handle.hugoPath }
          : {
              status: "starting",
              siteId: site.id,
            },
      );
    },
    onError: (e) => {
      setLifecycle({
        status: "error",
        siteId: site.id,
        message: describeError(e),
        tail: [],
      });
    },
  });

  const stop = useMutation({
    mutationFn: () => tauri.previewStop(),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["preview-status"] });
    },
  });

  const url = lifecycle.status === "running" ? lifecycle.url : null;
  const hugoPath =
    lifecycle.status === "running" ? lifecycle.hugoPath : undefined;

  return (
    <div className="flex h-full flex-col bg-background">
      <header className="flex items-center justify-between gap-2 border-b px-3 py-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
            Preview
          </span>
          <StatusDot lifecycle={lifecycle.status} />
          {url && (
            <span
              className="truncate font-mono text-[11px] text-muted-foreground"
              title={url}
            >
              {url}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {lifecycle.status === "running" ? (
            <>
              <Button
                size="icon"
                variant="ghost"
                onClick={() => setIframeKey((k) => k + 1)}
                aria-label="Reload preview"
                title="Reload"
              >
                <RotateCw className="size-4" />
              </Button>
              <Button
                size="icon"
                variant="ghost"
                onClick={() => stop.mutate()}
                disabled={stop.isPending}
                aria-label="Stop preview"
                title="Stop"
              >
                <Square className="size-4" />
              </Button>
            </>
          ) : (
            <Button
              size="sm"
              onClick={() => start.mutate()}
              disabled={start.isPending || lifecycle.status === "starting"}
            >
              <Play className="size-4" />
              {start.isPending || lifecycle.status === "starting"
                ? "Starting…"
                : "Start"}
            </Button>
          )}
          <Button
            size="icon"
            variant="ghost"
            onClick={onClose}
            aria-label="Close preview pane"
            title="Hide pane"
          >
            <X className="size-4" />
          </Button>
        </div>
      </header>

      <div className="flex-1 overflow-hidden bg-muted/20">
        {url ? (
          <iframe
            ref={iframeRef}
            key={iframeKey}
            src={url}
            title="Hugo live preview"
            className="h-full w-full border-0 bg-white"
          />
        ) : (
          <PreviewPlaceholder
            lifecycle={lifecycle.status}
            errorMessage={
              lifecycle.status === "error" ? lifecycle.message : null
            }
          />
        )}
      </div>

      {hugoPath && (
        <div className="border-t bg-muted/30 px-3 py-1 font-mono text-[10px] text-muted-foreground">
          hugo: {hugoPath}
        </div>
      )}

      <PreviewConsole open={consoleOpen} onOpenChange={setConsoleOpen} />
    </div>
  );
}

function StatusDot({ lifecycle }: { lifecycle: string }) {
  const tone =
    lifecycle === "running"
      ? "bg-emerald-500"
      : lifecycle === "starting"
        ? "bg-amber-500 animate-pulse"
        : lifecycle === "error"
          ? "bg-destructive"
          : "bg-muted-foreground/40";
  return (
    <span
      className={`inline-block size-2 rounded-full ${tone}`}
      title={lifecycle}
    />
  );
}

function PreviewPlaceholder({
  lifecycle,
  errorMessage,
}: {
  lifecycle: string;
  errorMessage: string | null;
}) {
  if (lifecycle === "starting") {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">Starting hugo server…</p>
      </div>
    );
  }
  if (lifecycle === "error") {
    return (
      <div className="flex h-full items-center justify-center px-6">
        <div className="max-w-md rounded-lg border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          <p className="font-medium">Preview failed</p>
          <p className="mt-1 break-all font-mono text-xs">
            {errorMessage ?? "unknown error"}
          </p>
          <p className="mt-2 text-xs text-destructive/80">
            See the Hugo console below for the tail of the output.
          </p>
        </div>
      </div>
    );
  }
  return (
    <div className="flex h-full items-center justify-center px-6">
      <div className="max-w-md text-center">
        <p className="text-sm font-medium">Preview is idle</p>
        <p className="mt-2 text-xs text-muted-foreground">
          Click <span className="font-medium">Start</span> to launch
          <code className="mx-1 font-mono">hugo server</code>
          on this site. Hugo must be installed on PATH (or pointed to via
          <code className="ml-1 font-mono">HUGO_STUDIO_HUGO_PATH</code>).
        </p>
      </div>
    </div>
  );
}
