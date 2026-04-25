import { useMemo } from "react";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { FieldDef, FrontMatterSchema, JsonValue } from "@/lib/tauri";
import { TagsInput } from "./TagsInput";

interface Props {
  schema: FrontMatterSchema;
  values: Record<string, JsonValue>;
  onChange: (next: Record<string, JsonValue>) => void;
}

/**
 * Schema-driven renderer. We don't bind through react-hook-form to keep
 * the controlled surface tiny: the editor owns the values and we just
 * forward setter calls. Validation lives in EditorView (zod-light).
 */
export function FrontMatterForm({ schema, values, onChange }: Props) {
  const groups = useMemo(() => groupBy(schema.fields), [schema.fields]);

  function setField(key: string, next: JsonValue) {
    const copy = { ...values };
    if (next === undefined || next === null || next === "") {
      delete copy[key];
    } else {
      copy[key] = next;
    }
    onChange(copy);
  }

  return (
    <div className="space-y-6">
      {groups.map(({ label, fields }) => (
        <fieldset key={label} className="space-y-3">
          <legend className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {label}
          </legend>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            {fields
              .filter((f) => !f.hidden)
              .map((field) => (
                <FieldRow
                  key={field.key}
                  field={field}
                  value={values[field.key]}
                  onChange={(next) => setField(field.key, next)}
                />
              ))}
          </div>
        </fieldset>
      ))}
    </div>
  );
}

function groupBy(fields: FieldDef[]) {
  const order: string[] = [];
  const buckets = new Map<string, FieldDef[]>();
  for (const f of fields) {
    const g = f.group ?? "Other";
    if (!buckets.has(g)) {
      buckets.set(g, []);
      order.push(g);
    }
    buckets.get(g)!.push(f);
  }
  return order.map((label) => ({ label, fields: buckets.get(label)! }));
}

function FieldRow({
  field,
  value,
  onChange,
}: {
  field: FieldDef;
  value: JsonValue | undefined;
  onChange: (next: JsonValue) => void;
}) {
  return (
    <label
      className={cn(
        "flex flex-col gap-1.5",
        field.fieldType === "boolean" && "rounded-md border p-3",
      )}
    >
      <span className="flex items-center justify-between text-sm font-medium">
        <span>
          {field.label}
          {field.required && <span className="ml-0.5 text-destructive">*</span>}
        </span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {field.key}
        </span>
      </span>
      <FieldWidget field={field} value={value} onChange={onChange} />
      {field.description && (
        <span className="text-xs text-muted-foreground">
          {field.description}
        </span>
      )}
    </label>
  );
}

function FieldWidget({
  field,
  value,
  onChange,
}: {
  field: FieldDef;
  value: JsonValue | undefined;
  onChange: (next: JsonValue) => void;
}) {
  switch (field.fieldType) {
    case "string":
      return (
        <Input
          type="text"
          value={asString(value)}
          onChange={(e) => onChange(e.target.value)}
        />
      );
    case "text":
      return (
        <textarea
          rows={3}
          value={asString(value)}
          onChange={(e) => onChange(e.target.value)}
          className="min-h-[80px] w-full resize-y rounded-md border border-input bg-background p-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
      );
    case "number":
      return (
        <Input
          type="number"
          value={value === undefined || value === null ? "" : String(value)}
          onChange={(e) => {
            const v = e.target.value;
            if (v === "") onChange(null);
            else onChange(Number(v));
          }}
        />
      );
    case "boolean":
      return (
        <input
          type="checkbox"
          checked={Boolean(value)}
          onChange={(e) => onChange(e.target.checked)}
          className="size-4 rounded border-input"
        />
      );
    case "date":
      return (
        <Input
          type="date"
          value={asDateString(value).slice(0, 10)}
          onChange={(e) => onChange(e.target.value || null)}
        />
      );
    case "dateTime":
      return (
        <Input
          type="datetime-local"
          value={asDateTimeLocal(value)}
          onChange={(e) => {
            const v = e.target.value;
            if (!v) {
              onChange(null);
              return;
            }
            // Hugo expects ISO 8601 in seconds resolution.
            onChange(`${v}:00`);
          }}
        />
      );
    case "tags":
      return (
        <TagsInput
          value={asStringArray(value)}
          onChange={(next) => onChange(next.length === 0 ? null : next)}
          suggestions={field.enumValues ?? []}
          placeholder={`Add ${field.label.toLowerCase()}…`}
        />
      );
    case "stringArray":
      return (
        <TagsInput
          value={asStringArray(value)}
          onChange={(next) => onChange(next.length === 0 ? null : next)}
          placeholder={`Add ${field.label.toLowerCase()}…`}
        />
      );
    case "json":
      return <JsonField value={value ?? null} onChange={onChange} />;
    default:
      return (
        <span className="text-xs text-destructive">unsupported field type</span>
      );
  }
}

function JsonField({
  value,
  onChange,
}: {
  value: JsonValue;
  onChange: (next: JsonValue) => void;
}) {
  const text = JSON.stringify(value, null, 2);
  return (
    <textarea
      rows={6}
      defaultValue={text}
      onBlur={(e) => {
        try {
          const parsed = JSON.parse(e.target.value);
          onChange(parsed);
        } catch {
          // Reject the edit silently — the user can fix and blur again.
        }
      }}
      spellCheck={false}
      className="min-h-[100px] w-full resize-y rounded-md border border-input bg-background p-2 font-mono text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    />
  );
}

function asString(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  return String(value);
}

function asStringArray(value: JsonValue | undefined): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((v): v is string => typeof v === "string");
}

function asDateString(value: JsonValue | undefined): string {
  if (typeof value === "string") return value;
  return "";
}

function asDateTimeLocal(value: JsonValue | undefined): string {
  if (typeof value !== "string" || value.length < 10) return "";
  // datetime-local wants `YYYY-MM-DDTHH:MM`; Hugo dates often arrive as ISO with
  // seconds + timezone. Trim what fits.
  const cleaned = value.replace(" ", "T");
  return cleaned.slice(0, 16);
}
