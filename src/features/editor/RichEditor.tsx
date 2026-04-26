import { useEffect, useRef } from "react";
import { Crepe } from "@milkdown/crepe";

import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import "@milkdown/crepe/theme/frame-dark.css";

interface Props {
  /** Initial markdown source. We snapshot it on mount and rebuild Crepe
   *  when the prop identity changes; live external updates are NOT
   *  pushed in (would fight the user's caret). To re-seed from outside,
   *  bump a `key` on this component. */
  value: string;
  onChange: (markdown: string) => void;
}

/** Milkdown's "Crepe" — a turnkey Markdown WYSIWYG with toolbar, slash
 *  menu, table tools, code blocks (CodeMirror inside), image blocks,
 *  link tooltips. Bidirectional with markdown source: the body string
 *  Hugo Studio holds is also what Milkdown emits via its listener.
 *
 *  Round-trip caveats (worth surfacing in the UI eventually):
 *    - Hugo shortcodes like `{{< youtube >}}` are passed through as
 *      raw text — they survive a round-trip but won't render.
 *    - Plain-text blocks may be normalised (e.g. `*foo*` ↔ `_foo_`,
 *      indentation in nested lists, soft vs hard breaks).
 *    - Front matter is intentionally not visible here — that's the
 *      "Front matter" tab.
 */
export function RichEditor({ value, onChange }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  // Stash the latest onChange in a ref so the effect can stay
  // mount-once without re-creating the editor on every render.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!containerRef.current) return;
    const crepe = new Crepe({
      root: containerRef.current,
      defaultValue: value,
    });
    let destroyed = false;
    crepe
      .create()
      .then(() => {
        if (destroyed) return;
        crepe.on((listener) => {
          listener.markdownUpdated((_ctx, md) => {
            onChangeRef.current(md);
          });
        });
      })
      .catch((err) => {
        console.error("[milkdown] failed to create editor", err);
      });
    return () => {
      destroyed = true;
      crepe.destroy();
    };
    // We deliberately mount once. The `value` prop is a *seed*, not a
    // controlled value — Crepe owns the editor state once mounted.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="milkdown-frame h-full overflow-auto bg-background">
      <div ref={containerRef} className="prose-container h-full" />
    </div>
  );
}
