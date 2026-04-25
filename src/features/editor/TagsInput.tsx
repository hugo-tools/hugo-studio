import { useEffect, useId, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";

import { cn } from "@/lib/utils";

interface Props {
  value: string[];
  onChange: (next: string[]) => void;
  /** Distinct values from the rest of the section — used as autocomplete. */
  suggestions?: string[];
  placeholder?: string;
}

/**
 * Chip-style tag input. The autocomplete list filters as the user types
 * and is keyboard-navigable (↑/↓/Enter/Tab adds the highlighted
 * suggestion; raw Enter on a free-form value adds it as-is).
 */
export function TagsInput({
  value,
  onChange,
  suggestions = [],
  placeholder,
}: Props) {
  const [draft, setDraft] = useState("");
  const [activeIdx, setActiveIdx] = useState(0);
  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listboxId = useId();

  const remaining = useMemo(
    () =>
      suggestions
        .filter((s) => !value.includes(s))
        .filter((s) => s.toLowerCase().includes(draft.trim().toLowerCase()))
        .slice(0, 8),
    [suggestions, value, draft],
  );

  useEffect(() => {
    setActiveIdx(0);
  }, [draft, open]);

  function commit(token: string) {
    const t = token.trim();
    if (!t) return;
    if (value.includes(t)) {
      setDraft("");
      return;
    }
    onChange([...value, t]);
    setDraft("");
  }

  function removeAt(idx: number) {
    onChange(value.filter((_, i) => i !== idx));
  }

  return (
    <div
      className="flex min-h-10 w-full flex-wrap items-center gap-1.5 rounded-md border border-input bg-background px-2 py-1.5 text-sm"
      onClick={() => inputRef.current?.focus()}
    >
      {value.map((tag, idx) => (
        <span
          key={`${tag}-${idx}`}
          className="inline-flex items-center gap-1 rounded bg-secondary px-2 py-0.5 text-xs text-secondary-foreground"
        >
          {tag}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              removeAt(idx);
            }}
            aria-label={`Remove ${tag}`}
            className="text-muted-foreground hover:text-foreground"
          >
            <X className="size-3" />
          </button>
        </span>
      ))}
      <div className="relative flex-1">
        <input
          ref={inputRef}
          value={draft}
          placeholder={value.length === 0 ? placeholder : undefined}
          onChange={(e) => {
            setDraft(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setTimeout(() => setOpen(false), 120)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === "," || e.key === "Tab") {
              if (open && remaining.length > 0 && draft.trim()) {
                e.preventDefault();
                commit(remaining[activeIdx] ?? draft);
              } else if (draft.trim()) {
                e.preventDefault();
                commit(draft);
              }
            } else if (e.key === "Backspace" && !draft && value.length > 0) {
              removeAt(value.length - 1);
            } else if (e.key === "ArrowDown" && remaining.length > 0) {
              e.preventDefault();
              setActiveIdx((i) => (i + 1) % remaining.length);
            } else if (e.key === "ArrowUp" && remaining.length > 0) {
              e.preventDefault();
              setActiveIdx(
                (i) => (i - 1 + remaining.length) % remaining.length,
              );
            } else if (e.key === "Escape") {
              setOpen(false);
            }
          }}
          aria-controls={listboxId}
          aria-autocomplete="list"
          className="w-full bg-transparent outline-none"
        />
        {open && remaining.length > 0 && (
          <ul
            id={listboxId}
            role="listbox"
            className="absolute left-0 top-full z-30 mt-1 max-h-56 min-w-[12rem] overflow-auto rounded-md border bg-popover p-1 text-sm shadow-md"
          >
            {remaining.map((s, idx) => (
              <li
                key={s}
                role="option"
                aria-selected={idx === activeIdx}
                onMouseDown={(e) => {
                  e.preventDefault();
                  commit(s);
                }}
                className={cn(
                  "cursor-pointer rounded px-2 py-1",
                  idx === activeIdx
                    ? "bg-accent text-accent-foreground"
                    : "hover:bg-accent",
                )}
              >
                {s}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
