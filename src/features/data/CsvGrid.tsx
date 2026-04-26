import { useEffect, useMemo, useRef, useState } from "react";
import { Plus, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { parseCsv, serialiseCsv } from "./csv";

interface Props {
  /** Raw CSV text from disk. */
  value: string;
  /** Called with the re-serialised CSV every time the grid changes. */
  onChange: (next: string) => void;
}

/** Default column width in pixels and the floor we clamp to so a
 *  user can't drag a column down to nothing and lose access to it. */
const DEFAULT_COL_WIDTH = 180;
const MIN_COL_WIDTH = 60;

/** Spreadsheet-style editor for CSV files. The first row is treated as
 *  headers (Hugo's `data/` convention); a Hugo template that loops over
 *  `index .Site.Data.foo` will see those header keys for each row. */
export function CsvGrid({ value, onChange }: Props) {
  // Maintain the parsed grid in local state so cell edits don't have
  // to reparse the entire CSV between keystrokes.
  const initial = useMemo(() => normalise(parseCsv(value)), [value]);
  const [headers, setHeaders] = useState<string[]>(initial.headers);
  const [rows, setRows] = useState<string[][]>(initial.rows);
  // Column widths in CSS pixels, parallel to `headers`. New columns
  // get DEFAULT_COL_WIDTH; resizing one cell sets only that column.
  const [colWidths, setColWidths] = useState<number[]>(() =>
    initial.headers.map(() => DEFAULT_COL_WIDTH),
  );

  // If the upstream `value` is replaced (e.g. after a Save round-trip
  // re-derives it) reseed local state — but only when the parsed shape
  // genuinely differs, so a save-induced refetch doesn't kill the
  // user's caret position on every change.
  useEffect(() => {
    const parsed = normalise(parseCsv(value));
    const same =
      JSON.stringify(parsed.headers) === JSON.stringify(headers) &&
      JSON.stringify(parsed.rows) === JSON.stringify(rows);
    if (!same) {
      setHeaders(parsed.headers);
      setRows(parsed.rows);
      // Preserve any user-resized widths that still apply (by index)
      // and pad out / trim the array to match the new column count.
      setColWidths((prev) => {
        const next = parsed.headers.map((_, i) => prev[i] ?? DEFAULT_COL_WIDTH);
        return next;
      });
    }
    // headers/rows intentionally omitted: this only resets when the
    // *input* changes, not when our edits do.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);

  // Column-resize drag state. We attach mousemove/mouseup to window
  // for the duration of the drag so a user releasing the mouse
  // outside the cell still ends the operation cleanly.
  const dragRef = useRef<{
    column: number;
    startX: number;
    startWidth: number;
  } | null>(null);

  function startResize(column: number, e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragRef.current = {
      column,
      startX: e.clientX,
      startWidth: colWidths[column] ?? DEFAULT_COL_WIDTH,
    };
    document.body.style.cursor = "col-resize";
    window.addEventListener("mousemove", onResizeMove);
    window.addEventListener("mouseup", onResizeEnd);
  }
  function onResizeMove(e: MouseEvent) {
    const drag = dragRef.current;
    if (!drag) return;
    const next = Math.max(
      MIN_COL_WIDTH,
      drag.startWidth + (e.clientX - drag.startX),
    );
    setColWidths((prev) => {
      const out = prev.slice();
      out[drag.column] = next;
      return out;
    });
  }
  function onResizeEnd() {
    dragRef.current = null;
    document.body.style.cursor = "";
    window.removeEventListener("mousemove", onResizeMove);
    window.removeEventListener("mouseup", onResizeEnd);
  }
  // Defensive cleanup if the component unmounts mid-drag.
  useEffect(() => {
    return () => {
      window.removeEventListener("mousemove", onResizeMove);
      window.removeEventListener("mouseup", onResizeEnd);
      document.body.style.cursor = "";
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function emit(nextHeaders: string[], nextRows: string[][]) {
    setHeaders(nextHeaders);
    setRows(nextRows);
    onChange(serialiseCsv([nextHeaders, ...nextRows]));
  }

  function setHeader(c: number, v: string) {
    const next = headers.slice();
    next[c] = v;
    emit(next, rows);
  }
  function setCell(r: number, c: number, v: string) {
    const nextRows = rows.map((row) => row.slice());
    while (nextRows[r].length < headers.length) nextRows[r].push("");
    nextRows[r][c] = v;
    emit(headers, nextRows);
  }
  function addRow() {
    emit(headers, [...rows, headers.map(() => "")]);
  }
  function removeRow(r: number) {
    emit(
      headers,
      rows.filter((_, i) => i !== r),
    );
  }
  function addColumn() {
    const nextHeaders = [...headers, `column${headers.length + 1}`];
    const nextRows = rows.map((row) => [...row, ""]);
    setColWidths((prev) => [...prev, DEFAULT_COL_WIDTH]);
    emit(nextHeaders, nextRows);
  }
  function removeColumn(c: number) {
    if (headers.length <= 1) return;
    const nextHeaders = headers.filter((_, i) => i !== c);
    const nextRows = rows.map((row) => row.filter((_, i) => i !== c));
    setColWidths((prev) => prev.filter((_, i) => i !== c));
    emit(nextHeaders, nextRows);
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex-1 overflow-auto">
        <table className="table-fixed border-separate border-spacing-0 text-xs">
          {/* `<colgroup>` drives the per-column widths; using inline
              `style.width` on a `<col>` lets us animate them via state
              without re-rendering each `<td>`. */}
          <colgroup>
            <col style={{ width: "2.5rem" }} />
            {headers.map((_h, c) => (
              <col
                key={c}
                style={{ width: `${colWidths[c] ?? DEFAULT_COL_WIDTH}px` }}
              />
            ))}
            <col style={{ width: "2.5rem" }} />
          </colgroup>
          <thead className="sticky top-0 z-10 bg-muted">
            <tr>
              <th className="border-b border-r px-2 py-1 text-[10px] text-muted-foreground">
                #
              </th>
              {headers.map((h, c) => (
                <th
                  key={c}
                  className="group relative border-b border-r px-1 py-1 text-left"
                >
                  <div className="flex items-center gap-1 pr-1">
                    <Input
                      type="text"
                      value={h}
                      onChange={(e) => setHeader(c, e.target.value)}
                      className="h-7 w-full min-w-0 flex-1 px-2 py-0 font-mono text-[11px] font-semibold"
                    />
                    <Button
                      type="button"
                      size="icon"
                      variant="ghost"
                      className={cn(
                        "size-6 shrink-0 text-destructive hover:text-destructive",
                        headers.length <= 1 && "invisible",
                      )}
                      onClick={() => removeColumn(c)}
                      title="Delete column"
                      aria-label={`Delete column ${h || c + 1}`}
                    >
                      <Trash2 className="size-3" />
                    </Button>
                  </div>
                  {/* Resize grip — 5px wide, sits over the right
                      border. The hover styling keeps it discoverable
                      without dominating the header layout. */}
                  <div
                    role="separator"
                    aria-label={`Resize column ${h || c + 1}`}
                    onMouseDown={(e) => startResize(c, e)}
                    className="absolute -right-px top-0 z-20 h-full w-1.5 cursor-col-resize select-none bg-transparent hover:bg-primary/40"
                  />
                </th>
              ))}
              <th className="border-b px-1 py-1">
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="size-6"
                  onClick={addColumn}
                  title="Add column"
                  aria-label="Add column"
                >
                  <Plus className="size-3.5" />
                </Button>
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row, r) => (
              <tr key={r} className="group hover:bg-muted/30">
                <td className="border-b border-r px-2 py-0.5 text-center text-[10px] text-muted-foreground">
                  {r + 1}
                </td>
                {headers.map((_h, c) => (
                  <td key={c} className="border-b border-r p-0.5">
                    <Input
                      type="text"
                      value={row[c] ?? ""}
                      onChange={(e) => setCell(r, c, e.target.value)}
                      className="h-7 w-full border-none px-2 py-0 font-mono text-[11px] focus-visible:ring-1"
                    />
                  </td>
                ))}
                <td className="border-b px-1 py-0.5">
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-6 text-destructive opacity-0 hover:text-destructive group-hover:opacity-100"
                    onClick={() => removeRow(r)}
                    title="Delete row"
                    aria-label={`Delete row ${r + 1}`}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="border-t bg-muted/20 px-3 py-2">
        <Button type="button" size="sm" variant="outline" onClick={addRow}>
          <Plus className="size-3.5" />
          Add row
        </Button>
      </div>
    </div>
  );
}

/** Pad rows out to the header width so cell access is bounds-safe.
 *  Empty CSVs get a single header column and one empty row so the
 *  grid has something to render. */
function normalise(parsed: string[][]): {
  headers: string[];
  rows: string[][];
} {
  if (parsed.length === 0) {
    return { headers: ["column1"], rows: [[""]] };
  }
  const headers = parsed[0].length > 0 ? parsed[0].slice() : ["column1"];
  const rows = parsed.slice(1).map((r) => {
    const padded = r.slice();
    while (padded.length < headers.length) padded.push("");
    return padded;
  });
  if (rows.length === 0) rows.push(headers.map(() => ""));
  return { headers, rows };
}
