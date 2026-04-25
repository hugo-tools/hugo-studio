import { Monitor, Moon, Sun } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useThemeStore, type ThemeMode } from "@/store/theme";

const ORDER: ThemeMode[] = ["system", "light", "dark"];

export function ThemeToggle() {
  const mode = useThemeStore((s) => s.mode);
  const setMode = useThemeStore((s) => s.setMode);

  function cycle() {
    const idx = ORDER.indexOf(mode);
    setMode(ORDER[(idx + 1) % ORDER.length]);
  }

  const Icon = mode === "dark" ? Moon : mode === "light" ? Sun : Monitor;
  const label =
    mode === "dark"
      ? "Dark mode"
      : mode === "light"
        ? "Light mode"
        : "System theme";

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      onClick={cycle}
      title={label}
      aria-label={label}
    >
      <Icon className={cn("size-4")} />
    </Button>
  );
}
