import { useEffect, useMemo, useState } from "react";
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

/** Spreadsheet-style editor for CSV files. The first row is treated as
 *  headers (Hugo's `data/` convention); a Hugo template that loops over
 *  `index .Site.Data.foo` will see those header keys for each row. */
export function CsvGrid({ value, onChange }: Props) {
  // Maintain the parsed grid in local state so cell edits don't have
  // to reparse the entire CSV between keystrokes.
  const initial = useMemo(() => normalise(parseCsv(value)), [value]);
  const [headers, setHeaders] = useState<string[]>(initial.headers);
  const [rows, setRows] = useState<string[][]>(initial.rows);

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
    }
    // headers/rows intentionally omitted: this only resets when the
    // *input* changes, not when our edits do.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);

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
    emit(nextHeaders, nextRows);
  }
  function removeColumn(c: number) {
    if (headers.length <= 1) return;
    const nextHeaders = headers.filter((_, i) => i !== c);
    const nextRows = rows.map((row) => row.filter((_, i) => i !== c));
    emit(nextHeaders, nextRows);
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex-1 overflow-auto">
        <table className="min-w-full table-fixed border-separate border-spacing-0 text-xs">
          <thead className="sticky top-0 z-10 bg-muted">
            <tr>
              <th className="w-10 border-b border-r px-2 py-1 text-[10px] text-muted-foreground">
                #
              </th>
              {headers.map((h, c) => (
                <th
                  key={c}
                  className="group min-w-[120px] border-b border-r px-1 py-1 text-left"
                >
                  <div className="flex items-center gap-1">
                    <Input
                      type="text"
                      value={h}
                      onChange={(e) => setHeader(c, e.target.value)}
                      className="h-7 w-full px-2 py-0 font-mono text-[11px] font-semibold"
                    />
                    <Button
                      type="button"
                      size="icon"
                      variant="ghost"
                      className={cn(
                        "size-6 text-destructive hover:text-destructive",
                        headers.length <= 1 && "invisible",
                      )}
                      onClick={() => removeColumn(c)}
                      title="Delete column"
                      aria-label={`Delete column ${h || c + 1}`}
                    >
                      <Trash2 className="size-3" />
                    </Button>
                  </div>
                </th>
              ))}
              <th className="w-10 border-b px-1 py-1">
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
