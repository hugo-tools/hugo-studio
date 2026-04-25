import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { EditorView } from "@codemirror/view";

interface Props {
  value: string;
  onChange: (next: string) => void;
}

export function BodyEditor({ value, onChange }: Props) {
  return (
    <CodeMirror
      value={value}
      height="100%"
      onChange={onChange}
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
}
