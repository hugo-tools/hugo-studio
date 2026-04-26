import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Save, Sparkles, FileCode2, Telescope } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  describeError,
  tauri,
  type JsonValue,
  type SchemaSource,
  type Site,
  type ThemeInfo,
} from "@/lib/tauri";
import { FrontMatterForm } from "@/features/editor/FrontMatterForm";

interface Props {
  site: Site;
}

export function ThemeSettingsPanel({ site }: Props) {
  const queryClient = useQueryClient();
  const info = useQuery<ThemeInfo>({
    queryKey: ["theme", site.id],
    queryFn: () => tauri.themeGet(site.id),
  });

  const [params, setParams] = useState<Record<string, JsonValue>>({});
  const [savedFlash, setSavedFlash] = useState(false);

  useEffect(() => {
    if (!info.data) return;
    setParams((info.data.params as Record<string, JsonValue>) ?? {});
  }, [info.data]);

  const dirty =
    !!info.data &&
    JSON.stringify(params) !== JSON.stringify(info.data.params ?? {});

  const save = useMutation({
    mutationFn: () => tauri.themeSaveParams(site.id, params as JsonValue),
    onSuccess: (next) => {
      queryClient.setQueryData(["theme", site.id], next);
      // The site config also changes (params lives there), so invalidate.
      queryClient.invalidateQueries({ queryKey: ["config", site.id] });
      setSavedFlash(true);
      setTimeout(() => setSavedFlash(false), 1800);
    },
  });

  if (info.isPending) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-muted-foreground">
        Loading theme…
      </p>
    );
  }
  if (info.isError || !info.data) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-destructive">
        {describeError(info.error)}
      </p>
    );
  }

  return (
    // Same shell every panel uses: header bar pinned to the top of the
    // tab area, content scrolls underneath. See SiteShell for context.
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between gap-3 border-b px-6 py-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h2 className="truncate text-sm font-semibold leading-tight">
              {info.data.themeName
                ? `Theme: ${info.data.themeName}`
                : "Theme params"}
            </h2>
            <SourceBadge source={info.data.source} />
          </div>
          {info.data.themePath && (
            <p
              className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground"
              title={info.data.themePath}
            >
              {info.data.themePath}
            </p>
          )}
          {!info.data.themeName && (
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              No <code className="font-mono">theme</code> set in your config —
              editing free-form params already in use.
            </p>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-3">
          {savedFlash && (
            <span className="text-xs text-muted-foreground">Saved.</span>
          )}
          {save.isError && (
            <span className="text-xs text-destructive">
              {describeError(save.error)}
            </span>
          )}
          <Button
            type="button"
            size="sm"
            disabled={!dirty || save.isPending}
            onClick={() => save.mutate()}
          >
            <Save className="size-4" />
            {save.isPending ? "Saving…" : "Save"}
          </Button>
        </div>
      </header>

      <div className="flex-1 overflow-auto px-6 py-6">
        {info.data.schema.fields.length === 0 ? (
          <div className="rounded-lg border border-dashed bg-muted/30 px-6 py-8 text-center text-sm text-muted-foreground">
            The theme exposes no params and the site has none set yet.
          </div>
        ) : (
          <FrontMatterForm
            schema={info.data.schema}
            values={params}
            onChange={setParams}
          />
        )}
      </div>
    </div>
  );
}

function SourceBadge({ source }: { source: SchemaSource }) {
  const detail = sourceDetail(source);
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider",
        detail.tone,
      )}
      title={detail.tooltip}
    >
      <detail.Icon className="size-3" />
      {detail.label}
    </span>
  );
}

function sourceDetail(source: SchemaSource) {
  switch (source) {
    case "manifest":
      return {
        label: "Manifest",
        tooltip:
          "Schema explicitly shipped by the theme at .hugoeditor/theme-schema.json — every field has the author's intent.",
        tone: "border-emerald-300 bg-emerald-50 text-emerald-900",
        Icon: FileCode2,
      };
    case "defaults":
      return {
        label: "Theme defaults",
        tooltip:
          "Inferred from the [params] section in the theme's config / theme.toml. Field types are guessed from the default values.",
        tone: "border-sky-300 bg-sky-50 text-sky-900",
        Icon: Sparkles,
      };
    case "inferred":
    default:
      return {
        label: "Inferred",
        tooltip:
          "No schema available — fields are inferred from the params currently set in your site. New fields the theme supports won't be visible until you set them at least once.",
        tone: "border-amber-300 bg-amber-50 text-amber-900",
        Icon: Telescope,
      };
  }
}
