// Minimal CSV parser / serialiser. Handles RFC 4180 quoting:
// fields containing comma, newline or quote get wrapped in `"..."`,
// embedded quotes are doubled. Accepts both LF and CRLF on input;
// emits LF on output (Hugo only cares that the file parses).

export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;
  let i = 0;
  const n = text.length;

  while (i < n) {
    const ch = text[i];
    if (inQuotes) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i += 2;
          continue;
        }
        inQuotes = false;
        i += 1;
        continue;
      }
      field += ch;
      i += 1;
    } else {
      if (ch === '"' && field === "") {
        inQuotes = true;
        i += 1;
        continue;
      }
      if (ch === ",") {
        row.push(field);
        field = "";
        i += 1;
        continue;
      }
      if (ch === "\r") {
        // Handle CRLF — fall through to the LF branch on the next char.
        i += 1;
        continue;
      }
      if (ch === "\n") {
        row.push(field);
        rows.push(row);
        row = [];
        field = "";
        i += 1;
        continue;
      }
      field += ch;
      i += 1;
    }
  }

  // Flush a trailing partial row (file ending without a newline) so we
  // don't silently drop the last record.
  if (field !== "" || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows;
}

export function serialiseCsv(rows: string[][]): string {
  if (rows.length === 0) return "";
  return rows.map((r) => r.map(escapeField).join(",")).join("\n") + "\n";
}

function escapeField(f: string): string {
  if (/[",\n\r]/.test(f)) {
    return `"${f.replace(/"/g, '""')}"`;
  }
  return f;
}
