import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, Plus, Save, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  describeError,
  tauri,
  type JsonValue,
  type LoadedConfig,
  type Site,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";

interface Props {
  site: Site;
}

interface MenuItem {
  identifier?: string;
  name: string;
  url: string;
  weight?: number;
  parent?: string;
}

/** Visual editor for Hugo's `[menu.<name>]` site config. Reads / writes
 *  through the existing config_get / config_save commands so the
 *  format-preserving codecs handle disk shape. */
export function MenuEditor({ site }: Props) {
  const queryClient = useQueryClient();
  const config = useQuery<LoadedConfig>({
    queryKey: ["config", site.id],
    queryFn: () => tauri.configGet(site.id),
  });

  const [draft, setDraft] = useState<Record<string, MenuItem[]> | null>(null);
  const [active, setActive] = useState<string | null>(null);

  // Seed the editing draft from the loaded config exactly once per
  // (re)load so the user's in-flight edits aren't blown away by the
  // background invalidation that follows a save.
  useEffect(() => {
    if (!config.data) return;
    if (draft !== null) return;
    const parsed = parseMenus(config.data.merged);
    setDraft(parsed);
    if (active === null) {
      const names = Object.keys(parsed);
      setActive(names[0] ?? null);
    }
  }, [config.data, draft, active]);

  const save = useMutation({
    mutationFn: async (next: Record<string, MenuItem[]>) => {
      if (!config.data) throw new Error("no config loaded");
      const merged = mergeMenusInto(config.data.merged, next);
      return tauri.configSave(site.id, merged);
    },
    onSuccess: (loaded) => {
      queryClient.setQueryData(["config", site.id], loaded);
      // Re-derive draft from the freshly written config so the user
      // sees normalised values (numeric weights, absent fields trimmed).
      setDraft(parseMenus(loaded.merged));
    },
    onError: (e) => alert(describeError(e)),
  });

  // Hooks first — every conditional return below comes AFTER them so
  // we don't violate the rules of hooks across loading/error states.
  const dirty = useMemoDirty(config.data, draft);

  if (config.isPending) {
    return <p className="px-6 py-10 text-sm text-muted-foreground">Loading…</p>;
  }
  if (config.isError) {
    return (
      <p className="px-6 py-10 text-sm text-destructive">
        Failed to load site config: {describeError(config.error)}
      </p>
    );
  }
  if (!draft) return null;

  // After the narrowing guard, alias to a non-null local so the
  // closures below don't have to re-prove it (TS doesn't carry the
  // narrowing across function boundaries).
  const menus = draft;
  const menuNames = Object.keys(menus).sort();

  const mutateActive = (updater: (items: MenuItem[]) => MenuItem[]) => {
    if (!active) return;
    setDraft((prev) => {
      if (!prev) return prev;
      return { ...prev, [active]: updater(prev[active] ?? []) };
    });
  };

  const createMenu = () => {
    const proposed = window.prompt(
      "New menu name (lowercase, e.g. 'main', 'footer')",
    );
    if (!proposed) return;
    const name = proposed.trim().toLowerCase();
    if (!name) return;
    if (menus[name]) {
      alert(`A menu named "${name}" already exists.`);
      setActive(name);
      return;
    }
    setDraft({ ...menus, [name]: [] });
    setActive(name);
  };

  const deleteMenu = (name: string) => {
    if (!confirm(`Delete the entire "${name}" menu?`)) return;
    const next = { ...menus };
    delete next[name];
    setDraft(next);
    setActive((cur) => (cur === name ? (Object.keys(next)[0] ?? null) : cur));
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between gap-3 border-b px-6 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <h2 className="text-sm font-semibold">Menus</h2>
          <span className="text-xs text-muted-foreground">
            Hugo `[menu.&lt;name&gt;]` site-config entries
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={createMenu}
          >
            <Plus className="size-3.5" />
            New menu
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={!dirty || save.isPending}
            onClick={() => save.mutate(menus)}
          >
            <Save className="size-3.5" />
            {save.isPending ? "Saving…" : "Save"}
          </Button>
        </div>
      </header>

      {menuNames.length === 0 ? (
        <p className="px-6 py-10 text-sm text-muted-foreground">
          No menus defined yet. Click "New menu" to add one.
        </p>
      ) : (
        <div className="flex flex-1 overflow-hidden">
          <aside className="w-48 shrink-0 overflow-auto border-r bg-muted/20 py-2">
            <ul className="space-y-0.5 px-1">
              {menuNames.map((name) => (
                <li key={name} className="group flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => setActive(name)}
                    className={cn(
                      "flex-1 truncate rounded px-2 py-1 text-left text-xs hover:bg-accent",
                      active === name && "bg-accent font-medium",
                    )}
                  >
                    {name}{" "}
                    <span className="text-[10px] text-muted-foreground">
                      ({menus[name].length})
                    </span>
                  </button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-6 opacity-0 group-hover:opacity-100"
                    onClick={() => deleteMenu(name)}
                    aria-label={`Delete menu ${name}`}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </li>
              ))}
            </ul>
          </aside>

          <section className="flex-1 overflow-auto p-6">
            {active && (
              <ItemTable
                items={menus[active] ?? []}
                allItems={menus[active] ?? []}
                onChange={mutateActive}
              />
            )}
          </section>
        </div>
      )}
    </div>
  );
}

function ItemTable({
  items,
  allItems,
  onChange,
}: {
  items: MenuItem[];
  allItems: MenuItem[];
  onChange: (updater: (items: MenuItem[]) => MenuItem[]) => void;
}) {
  function update(i: number, patch: Partial<MenuItem>) {
    onChange((prev) => {
      const next = prev.slice();
      next[i] = { ...next[i], ...patch };
      return next;
    });
  }
  function remove(i: number) {
    onChange((prev) => prev.filter((_, j) => j !== i));
  }
  function add() {
    onChange((prev) => [
      ...prev,
      {
        name: "New item",
        url: "/",
        weight: (prev.length + 1) * 10,
      },
    ]);
  }
  function move(i: number, dir: -1 | 1) {
    onChange((prev) => {
      const j = i + dir;
      if (j < 0 || j >= prev.length) return prev;
      const next = prev.slice();
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
  }

  // Sort by weight for display while preserving the underlying array
  // order — items with equal weights stay in their declared sequence.
  const ordered = items
    .map((it, i) => ({ it, i }))
    .sort((a, b) => (a.it.weight ?? 0) - (b.it.weight ?? 0));

  if (items.length === 0) {
    return (
      <div className="rounded border border-dashed p-8 text-center text-xs text-muted-foreground">
        No items yet.
        <div className="mt-3">
          <Button type="button" size="sm" variant="outline" onClick={add}>
            <Plus className="size-3.5" />
            Add item
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <table className="w-full table-auto text-xs">
        <thead>
          <tr className="text-left text-[10px] uppercase tracking-wider text-muted-foreground">
            <th className="px-2 py-1 font-medium">Name</th>
            <th className="px-2 py-1 font-medium">URL</th>
            <th className="px-2 py-1 font-medium">Weight</th>
            <th className="px-2 py-1 font-medium">Identifier</th>
            <th className="px-2 py-1 font-medium">Parent</th>
            <th className="w-20 px-2 py-1 font-medium">Order</th>
            <th className="w-8 px-2 py-1 font-medium" />
          </tr>
        </thead>
        <tbody>
          {ordered.map(({ it, i }) => (
            <tr key={i} className="border-t">
              <td className="px-1 py-1">
                <Input
                  type="text"
                  value={it.name}
                  onChange={(e) => update(i, { name: e.target.value })}
                  className="h-7 px-2 py-0 text-xs"
                />
              </td>
              <td className="px-1 py-1">
                <Input
                  type="text"
                  value={it.url}
                  onChange={(e) => update(i, { url: e.target.value })}
                  className="h-7 px-2 py-0 font-mono text-xs"
                />
              </td>
              <td className="px-1 py-1">
                <Input
                  type="number"
                  value={it.weight ?? ""}
                  onChange={(e) =>
                    update(i, {
                      weight:
                        e.target.value === ""
                          ? undefined
                          : Number(e.target.value),
                    })
                  }
                  className="h-7 w-20 px-2 py-0 text-xs"
                />
              </td>
              <td className="px-1 py-1">
                <Input
                  type="text"
                  value={it.identifier ?? ""}
                  onChange={(e) =>
                    update(i, {
                      identifier: e.target.value || undefined,
                    })
                  }
                  className="h-7 px-2 py-0 font-mono text-xs"
                />
              </td>
              <td className="px-1 py-1">
                <ParentSelect
                  value={it.parent ?? ""}
                  options={allItems
                    .map((p, j) => ({ p, j }))
                    .filter(({ j }) => j !== i)
                    .map(({ p }) => p.identifier ?? p.name)
                    .filter(Boolean)}
                  onChange={(v) => update(i, { parent: v || undefined })}
                />
              </td>
              <td className="px-1 py-1">
                <div className="flex gap-0.5">
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-6"
                    onClick={() => move(i, -1)}
                    aria-label="Move up"
                    title="Move up"
                  >
                    ↑
                  </Button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-6"
                    onClick={() => move(i, 1)}
                    aria-label="Move down"
                    title="Move down"
                  >
                    ↓
                  </Button>
                </div>
              </td>
              <td className="px-1 py-1">
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="size-7 text-destructive hover:text-destructive"
                  onClick={() => remove(i)}
                  aria-label={`Delete ${it.name}`}
                >
                  <Trash2 className="size-3.5" />
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <Button type="button" size="sm" variant="outline" onClick={add}>
        <Plus className="size-3.5" />
        Add item
      </Button>
    </div>
  );
}

function ParentSelect({
  value,
  options,
  onChange,
}: {
  value: string;
  options: string[];
  onChange: (v: string) => void;
}) {
  return (
    <div className="relative">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={cn(
          "h-7 w-full appearance-none rounded-md border border-input bg-transparent px-2 pr-7 text-xs",
          "focus:outline-none focus:ring-2 focus:ring-ring",
        )}
      >
        <option value="">(none)</option>
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </select>
      <ChevronDown className="pointer-events-none absolute right-1.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
    </div>
  );
}

/** Pull `menu.<name>: [...]` out of the merged config into our editing
 *  shape. Tolerates Hugo's two encodings: a single object per menu name
 *  (rare) or an array of objects (the common case). */
function parseMenus(merged: JsonValue): Record<string, MenuItem[]> {
  const out: Record<string, MenuItem[]> = {};
  if (!isObject(merged)) return out;
  const menuNode = (merged as Record<string, JsonValue>).menu;
  if (!isObject(menuNode)) return out;
  for (const [name, raw] of Object.entries(
    menuNode as Record<string, JsonValue>,
  )) {
    if (!Array.isArray(raw)) continue;
    out[name] = raw.filter(isObject).map((row) => {
      const r = row as Record<string, JsonValue>;
      return {
        name: typeof r.name === "string" ? r.name : "",
        url: typeof r.url === "string" ? r.url : "",
        weight: typeof r.weight === "number" ? r.weight : undefined,
        identifier: typeof r.identifier === "string" ? r.identifier : undefined,
        parent: typeof r.parent === "string" ? r.parent : undefined,
      };
    });
  }
  return out;
}

/** Replace `merged.menu` with the editor's current state. Items with
 *  empty names are dropped on the way out — the form lets the user
 *  blank them but Hugo would balk. */
function mergeMenusInto(
  merged: JsonValue,
  menus: Record<string, MenuItem[]>,
): JsonValue {
  const root = isObject(merged)
    ? { ...(merged as Record<string, JsonValue>) }
    : ({} as Record<string, JsonValue>);

  const menuOut: Record<string, JsonValue> = {};
  for (const [name, items] of Object.entries(menus)) {
    if (items.length === 0) continue;
    menuOut[name] = items
      .filter((it) => it.name.trim() !== "")
      .map((it) => {
        const row: Record<string, JsonValue> = {
          name: it.name,
          url: it.url,
        };
        if (it.weight !== undefined && Number.isFinite(it.weight)) {
          row.weight = it.weight;
        }
        if (it.identifier) row.identifier = it.identifier;
        if (it.parent) row.parent = it.parent;
        return row;
      });
  }
  if (Object.keys(menuOut).length === 0) {
    delete root.menu;
  } else {
    root.menu = menuOut;
  }
  return root;
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** Compare the current editing draft against the canonical merged
 *  config. Considered "dirty" when their JSON serialisations differ.
 *  Tolerates a `null` draft so the caller can keep this hook above
 *  any conditional return without violating the rules of hooks. */
function useMemoDirty(
  loaded: LoadedConfig | undefined,
  draft: Record<string, MenuItem[]> | null,
): boolean {
  return useMemo(() => {
    if (!loaded || !draft) return false;
    const onDisk = parseMenus(loaded.merged);
    return JSON.stringify(onDisk) !== JSON.stringify(draft);
  }, [loaded, draft]);
}
