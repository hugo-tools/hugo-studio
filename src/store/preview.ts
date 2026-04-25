import { create } from "zustand";

export type PreviewLifecycle =
  | { status: "idle" }
  | { status: "starting"; siteId: string }
  | {
      status: "running";
      siteId: string;
      url: string;
      port: number;
      hugoPath: string;
    }
  | { status: "error"; siteId: string; message: string; tail: string[] }
  | { status: "stopped"; siteId: string };

export interface LogLine {
  stream: "stdout" | "stderr";
  line: string;
  /** Wall-clock millis when the line was received — used for sorting + cap. */
  at: number;
}

interface PreviewState {
  lifecycle: PreviewLifecycle;
  logs: LogLine[];
  /** UI flag: whether the console drawer is expanded. */
  consoleOpen: boolean;
  setLifecycle: (
    next: PreviewLifecycle | ((cur: PreviewLifecycle) => PreviewLifecycle),
  ) => void;
  pushLog: (line: LogLine) => void;
  resetLogs: () => void;
  setConsoleOpen: (open: boolean) => void;
}

const LOG_CAP = 500;

export const usePreviewStore = create<PreviewState>((set) => ({
  lifecycle: { status: "idle" },
  logs: [],
  consoleOpen: false,
  setLifecycle: (next) =>
    set((s) => ({
      lifecycle: typeof next === "function" ? next(s.lifecycle) : next,
    })),
  pushLog: (line) =>
    set((s) => {
      const next = [...s.logs, line];
      if (next.length > LOG_CAP) next.splice(0, next.length - LOG_CAP);
      return { logs: next };
    }),
  resetLogs: () => set({ logs: [] }),
  setConsoleOpen: (consoleOpen) => set({ consoleOpen }),
}));
