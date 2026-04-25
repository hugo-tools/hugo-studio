import js from "@eslint/js";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsparser from "@typescript-eslint/parser";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";

const browserGlobals = {
  window: "readonly",
  document: "readonly",
  console: "readonly",
  navigator: "readonly",
  HTMLElement: "readonly",
  HTMLButtonElement: "readonly",
  HTMLInputElement: "readonly",
  HTMLDivElement: "readonly",
  HTMLAnchorElement: "readonly",
  HTMLFormElement: "readonly",
  HTMLTextAreaElement: "readonly",
  HTMLSpanElement: "readonly",
  Element: "readonly",
  Event: "readonly",
  KeyboardEvent: "readonly",
  MouseEvent: "readonly",
};

const nodeGlobals = {
  process: "readonly",
  require: "readonly",
  module: "readonly",
  __dirname: "readonly",
  __filename: "readonly",
};

export default [
  {
    ignores: [
      "dist",
      "src-tauri/target",
      "src-tauri/gen",
      "src/lib/tauri/bindings.ts",
    ],
  },
  js.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsparser,
      ecmaVersion: 2022,
      sourceType: "module",
      parserOptions: { ecmaFeatures: { jsx: true } },
      globals: { ...browserGlobals, ...nodeGlobals },
    },
    plugins: {
      "@typescript-eslint": tseslint,
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      ...reactHooks.configs.recommended.rules,
      // TypeScript handles no-undef itself; the JS rule false-positives DOM/Node types.
      "no-undef": "off",
      "react-refresh/only-export-components": [
        "warn",
        { allowConstantExport: true },
      ],
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
  {
    // shadcn/ui primitives co-locate the variants helper with the component.
    // Ignoring the fast-refresh warning on those files keeps the canonical layout.
    files: ["src/components/ui/**/*.{ts,tsx}"],
    rules: { "react-refresh/only-export-components": "off" },
  },
  {
    files: [
      "*.config.{js,ts,cjs,mjs}",
      "tailwind.config.js",
      "postcss.config.js",
    ],
    languageOptions: { globals: { ...nodeGlobals } },
  },
];
