import { forwardRef, useImperativeHandle, useRef } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { EditorView } from "@codemirror/view";

interface Props {
  value: string;
  onChange: (next: string) => void;
}

export interface BodyEditorHandle {
  /** Insert `text` at the current caret, leaving the caret after the
   *  inserted text. No-op if the editor isn't mounted yet. */
  insertAtCursor: (text: string) => void;
}

export const BodyEditor = forwardRef<BodyEditorHandle, Props>(
  function BodyEditor({ value, onChange }, ref) {
    const viewRef = useRef<EditorView | null>(null);

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
        extensions={[
          markdown(),
          EditorView.lineWrapping,
          EditorView.theme({
            "&": { height: "100%" },
            ".cm-scroller": {
              fontFamily:
                "ui-monospace, 'SF Mono', SFMono-Regular, Menlo, Consolas, monospace",
              fontSize: "13px",
              lineHeight: "1.6",
            },
            ".cm-content": { padding: "16px 20px" },
          }),
        ]}
        className="h-full"
      />
    );
  },
);
