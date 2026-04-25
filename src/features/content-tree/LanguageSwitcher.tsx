import { Languages } from "lucide-react";

import type { Language } from "@/lib/tauri";

interface Props {
  languages: Language[];
  active: string;
  onChange: (code: string) => void;
}

export function LanguageSwitcher({ languages, active, onChange }: Props) {
  if (languages.length <= 1) return null;
  return (
    <div className="flex items-center gap-2 text-xs">
      <Languages className="size-4 text-muted-foreground" />
      <select
        value={active}
        onChange={(e) => onChange(e.target.value)}
        className="rounded-md border bg-background px-2 py-1 text-xs"
      >
        {languages.map((lang) => (
          <option key={lang.code} value={lang.code}>
            {lang.name} ({lang.code})
          </option>
        ))}
      </select>
    </div>
  );
}
