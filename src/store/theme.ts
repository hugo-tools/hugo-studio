import { create } from "zustand";

export type ThemeMode = "light" | "dark" | "system";

const STORAGE_KEY = "hugo-studio.theme-mode";

interface ThemeState {
  mode: ThemeMode;
  setMode: (next: ThemeMode) => void;
  /** Resolve `system` to the OS preference at this moment. */
  effective: () => "light" | "dark";
  /** Push the effective class onto the <html> element. */
  apply: () => void;
}

function loadInitial(): ThemeMode {
  if (typeof window === "undefined") return "system";
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (raw === "light" || raw === "dark" || raw === "system") return raw;
  return "system";
}

function systemPrefersDark(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export const useThemeStore = create<ThemeState>((set, get) => ({
  mode: loadInitial(),
  setMode: (next) => {
    set({ mode: next });
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, next);
    }
    get().apply();
  },
  effective: () => {
    const m = get().mode;
    if (m === "system") return systemPrefersDark() ? "dark" : "light";
    return m;
  },
  apply: () => {
    if (typeof document === "undefined") return;
    const eff = get().effective();
    document.documentElement.classList.toggle("dark", eff === "dark");
  },
}));

/** Wire the OS-level scheme listener so `system` mode reacts live. */
export function bootstrapThemeMode() {
  if (typeof window === "undefined") return;
  useThemeStore.getState().apply();
  if (window.matchMedia) {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      if (useThemeStore.getState().mode === "system") {
        useThemeStore.getState().apply();
      }
    };
    if (mq.addEventListener) mq.addEventListener("change", onChange);
    else mq.addListener(onChange);
  }
}
