import { useState } from "react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { AssetContext } from "@/lib/tauri";

export type DropTarget = "bundle" | "static" | "assets";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  files: string[];
  /** True when the active content is a bundle (Branch/Leaf) — drives the
   *  default selection and whether the "bundle" choice is offered. */
  bundleAvailable: boolean;
  /** Hint label for the bundle option (e.g. "posts/hello/"). */
  bundleLabel: string | null;
  onConfirm: (context: AssetContext) => void;
}

export function AssetImportDialog({
  open,
  onOpenChange,
  files,
  bundleAvailable,
  bundleLabel,
  onConfirm,
}: Props) {
  const [target, setTarget] = useState<DropTarget>(
    bundleAvailable ? "bundle" : "static",
  );
  const [staticSubpath, setStaticSubpath] = useState("img");
  const [assetsSubpath, setAssetsSubpath] = useState("scss");

  function confirm() {
    let ctx: AssetContext;
    if (target === "bundle" && bundleAvailable) {
      ctx = { kind: "bundle", contentId: "__current__" };
    } else if (target === "static") {
      ctx = { kind: "static", subpath: staticSubpath.trim() };
    } else {
      ctx = { kind: "assets", subpath: assetsSubpath.trim() };
    }
    onConfirm(ctx);
  }

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Where should this go?</AlertDialogTitle>
          <AlertDialogDescription>
            {files.length === 1 ? (
              <>
                Importing{" "}
                <code className="font-mono">{baseName(files[0])}</code>
              </>
            ) : (
              <>
                Importing {files.length} files. They'll all land in the same
                place.
              </>
            )}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-2 py-2">
          <Choice
            id="bundle"
            label={
              bundleAvailable
                ? `Bundle — ${bundleLabel ?? "current page bundle"}`
                : "Bundle (only available when editing a Branch / Leaf bundle)"
            }
            description="Co-located with the page. Markdown can use a relative link like ![alt](file.jpg)."
            value="bundle"
            current={target}
            onChange={setTarget}
            disabled={!bundleAvailable}
          />
          <Choice
            id="static"
            label="Static — served as-is at the public URL"
            description={`Will land in static/<subpath>/ and be reachable at /<subpath>/<file>.`}
            value="static"
            current={target}
            onChange={setTarget}
          >
            <SubpathInput
              prefix="static/"
              value={staticSubpath}
              onChange={setStaticSubpath}
              disabled={target !== "static"}
            />
          </Choice>
          <Choice
            id="assets"
            label="Assets — Hugo Pipes input (SCSS, JS bundling, image processing)"
            description="Goes through Hugo's resource pipeline. Use it for SCSS / JS that needs minification, fingerprinting, or image transforms."
            value="assets"
            current={target}
            onChange={setTarget}
          >
            <SubpathInput
              prefix="assets/"
              value={assetsSubpath}
              onChange={setAssetsSubpath}
              disabled={target !== "assets"}
            />
          </Choice>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={(e) => {
              e.preventDefault();
              confirm();
            }}
          >
            Import
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function Choice({
  id,
  label,
  description,
  value,
  current,
  onChange,
  disabled,
  children,
}: {
  id: string;
  label: string;
  description: string;
  value: DropTarget;
  current: DropTarget;
  onChange: (next: DropTarget) => void;
  disabled?: boolean;
  children?: React.ReactNode;
}) {
  return (
    <label
      htmlFor={id}
      className={cn(
        "flex cursor-pointer flex-col gap-1 rounded-md border p-3 text-sm",
        current === value && !disabled && "border-primary bg-primary/5",
        disabled && "cursor-not-allowed opacity-60",
      )}
    >
      <span className="flex items-center gap-2">
        <input
          id={id}
          type="radio"
          name="drop-target"
          value={value}
          checked={current === value}
          onChange={() => onChange(value)}
          disabled={disabled}
        />
        <span className="font-medium">{label}</span>
      </span>
      <span className="ml-6 text-xs text-muted-foreground">{description}</span>
      {children && <div className="ml-6">{children}</div>}
    </label>
  );
}

function SubpathInput({
  prefix,
  value,
  onChange,
  disabled,
}: {
  prefix: string;
  value: string;
  onChange: (next: string) => void;
  disabled?: boolean;
}) {
  return (
    <div className="mt-2 flex items-center gap-1 font-mono text-xs">
      <span className="text-muted-foreground">{prefix}</span>
      <Input
        type="text"
        className="h-7 flex-1 px-2 py-0 text-xs"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder="(empty for root)"
      />
    </div>
  );
}

function baseName(p: string): string {
  const idx = p.lastIndexOf("/");
  const idxBs = p.lastIndexOf("\\");
  const cut = Math.max(idx, idxBs);
  return cut >= 0 ? p.slice(cut + 1) : p;
}
