import { useEffect, useMemo, useState } from "react";
import { useForm, type SubmitHandler } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Save } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  describeError,
  tauri,
  type JsonValue,
  type LoadedConfig,
  type Site,
} from "@/lib/tauri";
import {
  knownFieldDefs,
  knownFieldKeys,
  knownFieldsSchema,
  type KnownFieldsValues,
} from "./knownFields";

interface Props {
  site: Site;
}

export function SiteSettingsPanel({ site }: Props) {
  const queryClient = useQueryClient();
  const [savedFlash, setSavedFlash] = useState(false);

  const config = useQuery<LoadedConfig>({
    queryKey: ["config", site.id],
    queryFn: () => tauri.configGet(site.id),
  });

  const form = useForm<KnownFieldsValues>({
    resolver: zodResolver(knownFieldsSchema),
    defaultValues: extractKnown(config.data?.merged),
  });

  // Re-seed the form when the underlying config arrives or changes.
  useEffect(() => {
    form.reset(extractKnown(config.data?.merged));
  }, [config.data, form]);

  const save = useMutation({
    mutationFn: async (values: KnownFieldsValues) => {
      if (!config.data) throw new Error("config not loaded yet");
      const merged = mergeKnown(config.data.merged, values);
      return tauri.configSave(site.id, merged as JsonValue);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config", site.id] });
      setSavedFlash(true);
      setTimeout(() => setSavedFlash(false), 1800);
    },
  });

  const onSubmit: SubmitHandler<KnownFieldsValues> = (values) =>
    save.mutate(values);

  const advancedJson = useMemo(() => {
    if (!config.data) return "";
    const obj = (config.data.merged as Record<string, unknown>) ?? {};
    const advancedEntries = Object.entries(obj).filter(
      ([k]) => !knownFieldKeys.includes(k),
    );
    return JSON.stringify(Object.fromEntries(advancedEntries), null, 2);
  }, [config.data]);

  if (config.isPending) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-muted-foreground">
        Loading config…
      </p>
    );
  }

  if (config.isError) {
    return (
      <p className="px-6 pb-10 pt-4 text-sm text-destructive">
        Failed to load config: {describeError(config.error)}
      </p>
    );
  }

  return (
    // Same shell every panel uses: header bar pinned to the top of the
    // tab area, content scrolls underneath. Keeps tab switching from
    // looking like the start position is jumping around.
    <form
      onSubmit={form.handleSubmit(onSubmit)}
      className="flex h-full flex-col"
    >
      <header className="flex items-center justify-between gap-3 border-b px-6 py-3">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold leading-tight">Site</h2>
          <p
            className="truncate text-xs text-muted-foreground"
            title={config.data?.sources.map((s) => s.path).join(", ")}
          >
            Writes to{" "}
            <code className="font-mono">
              {config.data?.sources.map((s) => relName(s.path)).join(", ")}
            </code>{" "}
            · {config.data?.format.toUpperCase()}
          </p>
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
            type="submit"
            size="sm"
            disabled={save.isPending || !form.formState.isDirty}
          >
            <Save className="size-4" />
            {save.isPending ? "Saving…" : "Save changes"}
          </Button>
        </div>
      </header>

      <div className="flex-1 space-y-8 overflow-auto px-6 py-6">
        <section>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            {knownFieldDefs.map((field) => (
              <FormField key={field.key} field={field} form={form} />
            ))}
          </div>
        </section>

        <section>
          <h2 className="mb-2 text-base font-semibold">Advanced (read-only)</h2>
          <p className="mb-3 text-xs text-muted-foreground">
            Top-level keys outside the curated form. JSON-editing arbitrary keys
            is M9 work.
          </p>
          <pre className="max-h-80 overflow-auto rounded-md border bg-muted/30 p-4 font-mono text-xs leading-relaxed">
            {advancedJson || "{}"}
          </pre>
        </section>
      </div>
    </form>
  );
}

function FormField({
  field,
  form,
}: {
  field: (typeof knownFieldDefs)[number];
  form: ReturnType<typeof useForm<KnownFieldsValues>>;
}) {
  const error = form.formState.errors[field.key as keyof KnownFieldsValues];

  if (field.type === "boolean") {
    return (
      <label className="flex cursor-pointer items-start gap-3 rounded-md border p-3">
        <input
          type="checkbox"
          {...form.register(field.key as keyof KnownFieldsValues)}
          className="mt-0.5 size-4 rounded border-input"
        />
        <div>
          <div className="text-sm font-medium">{field.label}</div>
          <div className="text-xs text-muted-foreground">
            Hugo key: <code className="font-mono">{field.key}</code>
          </div>
        </div>
      </label>
    );
  }

  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-sm font-medium">{field.label}</span>
      <Input
        type={field.type === "number" ? "number" : "text"}
        placeholder={field.placeholder}
        {...form.register(field.key as keyof KnownFieldsValues)}
      />
      <span className="text-xs text-muted-foreground">
        Hugo key: <code className="font-mono">{field.key}</code>
        {"hint" in field && field.hint ? ` — ${field.hint}` : ""}
      </span>
      {error && (
        <span className="text-xs text-destructive">
          {error.message?.toString()}
        </span>
      )}
    </label>
  );
}

function extractKnown(merged: unknown): KnownFieldsValues {
  const obj = (merged as Record<string, unknown> | undefined) ?? {};
  return {
    title: typeof obj.title === "string" ? obj.title : undefined,
    baseURL: typeof obj.baseURL === "string" ? obj.baseURL : undefined,
    languageCode:
      typeof obj.languageCode === "string" ? obj.languageCode : undefined,
    defaultContentLanguage:
      typeof obj.defaultContentLanguage === "string"
        ? obj.defaultContentLanguage
        : undefined,
    theme: typeof obj.theme === "string" ? obj.theme : undefined,
    paginate:
      typeof obj.paginate === "number"
        ? obj.paginate
        : typeof obj.paginate === "string" && obj.paginate !== ""
          ? Number(obj.paginate)
          : undefined,
    enableEmoji:
      typeof obj.enableEmoji === "boolean" ? obj.enableEmoji : undefined,
    enableRobotsTXT:
      typeof obj.enableRobotsTXT === "boolean"
        ? obj.enableRobotsTXT
        : undefined,
  };
}

function mergeKnown(merged: unknown, values: KnownFieldsValues): unknown {
  const base = { ...((merged as Record<string, unknown>) ?? {}) };
  for (const [key, value] of Object.entries(values)) {
    if (value === undefined || value === "") {
      delete base[key];
    } else {
      base[key] = value;
    }
  }
  return base;
}

function relName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts.slice(-2).join("/");
}
