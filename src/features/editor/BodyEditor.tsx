import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { html } from "@codemirror/lang-html";
import { json } from "@codemirror/lang-json";
import { EditorView } from "@codemirror/view";

import { useThemeStore } from "@/store/theme";

export type BodyLanguage = "markdown" | "html" | "json";

interface Props {
  value: string;
  onChange: (next: string) => void;
  language?: BodyLanguage;
}

export interface BodyEditorHandle {
  /** Insert `text` at the current caret, leaving the caret after the
   *  inserted text. No-op if the editor isn't mounted yet. */
  insertAtCursor: (text: string) => void;
}

/** Resolve "system" mode to dark/light by querying matchMedia. The
 *  hook re-renders when the OS preference flips so the editor flips
 *  too without a manual toggle. */
function useEffectiveDark(): boolean {
  const mode = useThemeStore((s) => s.mode);
  const [systemDark, setSystemDark] = useState<boolean>(() =>
    typeof window !== "undefined" && window.matchMedia
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : false,
  );
  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    if (mq.addEventListener) mq.addEventListener("change", onChange);
    else mq.addListener(onChange);
    return () => {
      if (mq.removeEventListener) mq.removeEventListener("change", onChange);
      else mq.removeListener(onChange);
    };
  }, []);
  if (mode === "dark") return true;
  if (mode === "light") return false;
  return systemDark;
}

/** Build a CodeMirror theme bound to our Tailwind CSS variables.
 *  Passing `dark: true` flips CodeMirror's internal base styles
 *  (cursor / selection contrast) while we keep visible colours in
 *  sync with the rest of the app via `--background` / `--foreground`
 *  / `--accent` / `--muted`. */
function buildAppTheme(dark: boolean) {
  return EditorView.theme(
    {
      "&": {
        height: "100%",
        backgroundColor: "hsl(var(--background))",
        color: "hsl(var(--foreground))",
      },
      ".cm-scroller": {
        backgroundColor: "transparent",
        color: "inherit",
        fontFamily:
          "ui-monospace, 'SF Mono', SFMono-Regular, Menlo, Consolas, monospace",
        fontSize: "13px",
        lineHeight: "1.6",
      },
      ".cm-content": {
        backgroundColor: "transparent",
        color: "hsl(var(--foreground))",
        caretColor: "hsl(var(--foreground))",
        padding: "16px 20px",
      },
      ".cm-line": { color: "inherit" },
      ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: "hsl(var(--foreground))",
      },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "hsl(var(--accent))" },
      ".cm-gutters": {
        backgroundColor: "hsl(var(--muted))",
        color: "hsl(var(--muted-foreground))",
        border: "none",
      },
      ".cm-activeLine, .cm-activeLineGutter": {
        backgroundColor: "hsl(var(--accent) / 0.2)",
      },
    },
    { dark },
  );
}

export const BodyEditor = forwardRef<BodyEditorHandle, Props>(
  function BodyEditor({ value, onChange, language = "markdown" }, ref) {
    const viewRef = useRef<EditorView | null>(null);
    const isDark = useEffectiveDark();

    useImperativeHandle(
      ref,
      () => ({
        insertAtCursor(text: string) {
          const view = viewRef.current;
          if (!view) return;
          const { from } = view.state.selection.main;
          view.dispatch({
            changes: { from, insert: text },
            selection: { anchor: from + text.length },
          });
          view.focus();
        },
      }),
      [],
    );

    const extensions = useMemo(() => {
      const lang =
        language === "html"
          ? html()
          : language === "json"
            ? json()
            : markdown();
      return [lang, EditorView.lineWrapping, buildAppTheme(isDark)];
    }, [isDark, language]);

    return (
      <CodeMirror
        value={value}
        height="100%"
        onChange={onChange}
        onCreateEditor={(view) => {
          viewRef.current = view;
        }}
        basicSetup={{
          lineNumbers: false,
          foldGutter: false,
          highlightActiveLine: false,
          highlightActiveLineGutter: false,
        }}
        // `theme="none"` disables the built-in `'light'` theme that
        // @uiw/react-codemirror applies by default — the real fix for
        // the white-on-white dark-mode bug. Our buildAppTheme() above
        // is the only theme actually in effect.
        theme="none"
        extensions={extensions}
        className="h-full"
      />
    );
  },
);
