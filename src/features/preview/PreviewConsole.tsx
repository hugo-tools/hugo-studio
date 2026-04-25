import { useEffect, useRef } from "react";
import { ChevronDown, ChevronUp, Eraser } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { usePreviewStore } from "@/store/preview";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function PreviewConsole({ open, onOpenChange }: Props) {
  const logs = usePreviewStore((s) => s.logs);
  const resetLogs = usePreviewStore((s) => s.resetLogs);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom on new log lines, unless the user scrolled up.
  useEffect(() => {
    if (!open || !scrollRef.current) return;
    const el = scrollRef.current;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    if (nearBottom) el.scrollTop = el.scrollHeight;
  }, [logs, open]);

  return (
    <div className="border-t bg-background">
      <button
        type="button"
        onClick={() => onOpenChange(!open)}
        className="flex w-full items-center justify-between px-4 py-1.5 text-xs font-medium uppercase tracking-wider text-muted-foreground hover:bg-muted/50"
      >
        <span className="flex items-center gap-2">
          {open ? (
            <ChevronDown className="size-3" />
          ) : (
            <ChevronUp className="size-3" />
          )}
          Hugo console · {logs.length} line{logs.length === 1 ? "" : "s"}
        </span>
        {open && logs.length > 0 && (
          <Button
            asChild
            size="sm"
            variant="ghost"
            onClick={(e) => {
              e.stopPropagation();
              resetLogs();
            }}
          >
            <span className="cursor-pointer">
              <Eraser className="size-3" />
              Clear
            </span>
          </Button>
        )}
      </button>
      {open && (
        <div
          ref={scrollRef}
          className="max-h-56 overflow-auto bg-muted/30 px-4 py-2 font-mono text-[11px] leading-relaxed"
        >
          {logs.length === 0 ? (
            <p className="text-muted-foreground">No output yet.</p>
          ) : (
            logs.map((l, i) => (
              <div
                key={`${l.at}-${i}`}
                className={cn(
                  "whitespace-pre-wrap break-all",
                  l.stream === "stderr"
                    ? "text-amber-700 dark:text-amber-400"
                    : "text-foreground/80",
                )}
              >
                {l.line}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
