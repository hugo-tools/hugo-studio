import { z } from "zod";

/**
 * Top-level Hugo config fields that get a curated form widget. Anything
 * else lands in the "Advanced" read-only JSON viewer for now (M2 scope —
 * a JSON editor for arbitrary keys is a follow-up).
 */
export const knownFieldDefs = [
  {
    key: "title",
    label: "Site title",
    type: "string" as const,
    placeholder: "My Hugo Site",
  },
  {
    key: "baseURL",
    label: "Base URL",
    type: "string" as const,
    placeholder: "https://example.com/",
    hint: "Public root of the site, including trailing slash.",
  },
  {
    key: "languageCode",
    label: "Language code",
    type: "string" as const,
    placeholder: "en-us",
  },
  {
    key: "defaultContentLanguage",
    label: "Default content language",
    type: "string" as const,
    placeholder: "en",
  },
  {
    key: "theme",
    label: "Theme",
    type: "string" as const,
    placeholder: "papermod",
  },
  {
    key: "paginate",
    label: "Items per page",
    type: "number" as const,
    placeholder: "10",
  },
  { key: "enableEmoji", label: "Enable emoji", type: "boolean" as const },
  {
    key: "enableRobotsTXT",
    label: "Generate robots.txt",
    type: "boolean" as const,
  },
] as const;

export type KnownFieldKey = (typeof knownFieldDefs)[number]["key"];

export const knownFieldKeys: readonly string[] = knownFieldDefs.map(
  (f) => f.key,
);

/**
 * Schema for the form. Every field is optional because Hugo treats missing
 * keys as "use the framework default"; we don't want the editor to invent
 * defaults that didn't exist in the user's source.
 */
export const knownFieldsSchema = z.object({
  title: z.string().optional(),
  baseURL: z.string().optional(),
  languageCode: z.string().optional(),
  defaultContentLanguage: z.string().optional(),
  theme: z.string().optional(),
  paginate: z
    .union([z.coerce.number().int().nonnegative(), z.literal("")])
    .optional()
    .transform((v) => (v === "" || v === undefined ? undefined : v)),
  enableEmoji: z.boolean().optional(),
  enableRobotsTXT: z.boolean().optional(),
});

export type KnownFieldsValues = z.infer<typeof knownFieldsSchema>;
