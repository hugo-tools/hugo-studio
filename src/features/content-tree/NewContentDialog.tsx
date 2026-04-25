import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FilePlus2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import {
  describeError,
  tauri,
  type Archetype,
  type ContentScanResult,
  type Language,
  type Site,
} from "@/lib/tauri";
import { useWorkspaceStore } from "@/store/workspace";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  site: Site;
  scan: ContentScanResult | null;
  /** Optional pre-selected section (e.g. user clicked a folder before "New"). */
  initialSection?: string;
}

function slugify(input: string): string {
  return input
    .normalize("NFKD")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80);
}

export function NewContentDialog({
  open,
  onOpenChange,
  site,
  scan,
  initialSection,
}: Props) {
  const queryClient = useQueryClient();
  const selectContent = useWorkspaceStore((s) => s.selectContent);

  const archetypes = useQuery<Archetype[]>({
    queryKey: ["archetypes", site.id],
    queryFn: () => tauri.contentArchetypes(site.id),
    enabled: open,
  });

  const sections = useMemo(() => {
    const set = new Set<string>();
    scan?.items.forEach((i) => {
      if (i.section) set.add(i.section);
    });
    return Array.from(set).sort();
  }, [scan]);

  const languages: Language[] = scan?.languageInfo.languages ?? [];

  const [section, setSection] = useState(initialSection ?? "");
  const [titleInput, setTitleInput] = useState("");
  const [archetypeName, setArchetypeName] = useState<string>("(auto)");
  const [language, setLanguage] = useState<string>(
    scan?.languageInfo.defaultLanguage ?? "",
  );

  // Reseed when the dialog re-opens.
  useEffect(() => {
    if (!open) return;
    setSection(initialSection ?? sections[0] ?? "posts");
    setTitleInput("");
    setArchetypeName("(auto)");
    setLanguage(scan?.languageInfo.defaultLanguage ?? "");
  }, [open, initialSection, sections, scan]);

  const slug = slugify(titleInput);

  const create = useMutation({
    mutationFn: () =>
      tauri.contentCreate(site.id, {
        section,
        slug,
        archetype: archetypeName === "(auto)" ? null : archetypeName,
        language: languages.length > 0 ? language || null : null,
      }),
    onSuccess: (created) => {
      queryClient.invalidateQueries({ queryKey: ["content", site.id] });
      onOpenChange(false);
      // Open the freshly-created content in the editor.
      selectContent({
        path: created.path,
        id: created.id,
        language:
          created.language ?? scan?.languageInfo.defaultLanguage ?? "en",
        title: titleInput,
      });
    },
  });

  const archetypeOptions = [
    "(auto)",
    ...(archetypes.data?.map((a) => a.name) ?? []),
  ];

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>New content</AlertDialogTitle>
          <AlertDialogDescription>
            The slug is derived from the title — adjust the title to change the
            file name.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-3 py-2">
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium">Title</span>
            <Input
              type="text"
              value={titleInput}
              onChange={(e) => setTitleInput(e.target.value)}
              placeholder="My first post"
              autoFocus
            />
            {slug && (
              <span className="font-mono text-xs text-muted-foreground">
                slug: {slug}
              </span>
            )}
          </label>

          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium">Section</span>
            <div className="flex gap-2">
              <Input
                type="text"
                list="section-suggestions"
                value={section}
                onChange={(e) => setSection(e.target.value)}
                placeholder="posts"
              />
              <datalist id="section-suggestions">
                {sections.map((s) => (
                  <option key={s} value={s} />
                ))}
              </datalist>
            </div>
          </label>

          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium">Archetype</span>
            <select
              value={archetypeName}
              onChange={(e) => setArchetypeName(e.target.value)}
              className="h-10 rounded-md border border-input bg-background px-3 text-sm"
            >
              {archetypeOptions.map((name) => (
                <option key={name} value={name}>
                  {name}
                  {name === "(auto)"
                    ? " — section, then default, then built-in"
                    : ""}
                </option>
              ))}
            </select>
          </label>

          {languages.length > 1 && (
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="font-medium">Language</span>
              <select
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
                className="h-10 rounded-md border border-input bg-background px-3 text-sm"
              >
                {languages.map((lang) => (
                  <option key={lang.code} value={lang.code}>
                    {lang.name} ({lang.code})
                  </option>
                ))}
              </select>
            </label>
          )}

          {create.isError && (
            <p className="text-xs text-destructive">
              {describeError(create.error)}
            </p>
          )}
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={create.isPending}>
            Cancel
          </AlertDialogCancel>
          <Button
            type="button"
            onClick={() => create.mutate()}
            disabled={!section.trim() || !slug || create.isPending}
          >
            <FilePlus2 className="size-4" />
            {create.isPending ? "Creating…" : "Create"}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
