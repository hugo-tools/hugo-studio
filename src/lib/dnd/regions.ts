// Module-level registry of "drop regions" — DOM areas that want to
// claim native (OS) drag-drop events from Tauri's webview-wide
// `onDragDropEvent` so a fallback handler can route to a default UI
// (e.g. the editor's import dialog) when no region matches.
//
// Why this exists: Tauri 2 captures OS drops at the webview boundary
// (HTML5 drag-drop is suppressed on most platforms). The single event
// stream carries a position; we use it to decide which on-screen
// region the drop landed on.

export interface DropPosition {
  /** Webview-relative coordinates in physical pixels (Tauri's payload). */
  x: number;
  y: number;
}

export interface DropRegion {
  /** Return true to handle the drop. Receives CSS-pixel coordinates. */
  match: (cssPos: { x: number; y: number }) => boolean;
  /** Called when this region wins; receives the dropped absolute paths. */
  handle: (paths: string[]) => void | Promise<void>;
  /** Optional priority — higher numbers tested first (default 0). Used so
   *  modals/overlays can win over panels behind them. */
  priority?: number;
}

const regions = new Set<DropRegion>();

/** Register a region. Returns an unregister function. */
export function registerDropRegion(r: DropRegion): () => void {
  regions.add(r);
  return () => {
    regions.delete(r);
  };
}

/** Try to dispatch a drop to a registered region. Returns true if any
 *  region matched. The first highest-priority match wins. */
export function tryDispatchDrop(pos: DropPosition, paths: string[]): boolean {
  if (regions.size === 0) return false;
  const cssPos = toCssPixels(pos);
  const sorted = Array.from(regions).sort(
    (a, b) => (b.priority ?? 0) - (a.priority ?? 0),
  );
  for (const r of sorted) {
    if (r.match(cssPos)) {
      void r.handle(paths);
      return true;
    }
  }
  return false;
}

/** Tauri reports physical pixels; the DOM works in CSS pixels. Divide by
 *  `devicePixelRatio` to compare against `getBoundingClientRect()`. On
 *  retina displays this is the 2x factor; on Linux X11 it depends on
 *  the user's DPI scaling. */
function toCssPixels(p: DropPosition): { x: number; y: number } {
  const dpr =
    typeof window !== "undefined" && window.devicePixelRatio
      ? window.devicePixelRatio
      : 1;
  return { x: p.x / dpr, y: p.y / dpr };
}

/** Helper: build a `match` closure that hit-tests against a DOM node's
 *  current bounding rect. Recomputes on each call so layout changes
 *  (tab switches, resizes, scrolls of the window) don't stale-cache. */
export function rectMatcher(
  getEl: () => HTMLElement | null,
): (cssPos: { x: number; y: number }) => boolean {
  return (cssPos) => {
    const el = getEl();
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return (
      cssPos.x >= r.left &&
      cssPos.x <= r.right &&
      cssPos.y >= r.top &&
      cssPos.y <= r.bottom
    );
  };
}
