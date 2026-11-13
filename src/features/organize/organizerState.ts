// Pure state logic for the page organizer, kept separate for testing.

export interface PageItem {
  /** Stable id for React and drag-and-drop. */
  uid: string;
  /** 1-based page number in the source document. */
  page: number;
  /** Extra clockwise rotation in degrees (0, 90, 180, 270). */
  rotate: number;
}

export interface OrganizerState {
  items: PageItem[];
  past: PageItem[][];
  future: PageItem[][];
}

let counter = 0;
export const newUid = () => `p${++counter}-${Math.random().toString(36).slice(2, 7)}`;

export function initialItems(pageCount: number): PageItem[] {
  return Array.from({ length: pageCount }, (_, i) => ({ uid: newUid(), page: i + 1, rotate: 0 }));
}

export function initialState(pageCount: number): OrganizerState {
  return { items: initialItems(pageCount), past: [], future: [] };
}

const HISTORY_LIMIT = 100;

/** Records a change so it can be undone. */
export function commit(state: OrganizerState, items: PageItem[]): OrganizerState {
  if (items === state.items) return state;
  return { items, past: [...state.past, state.items].slice(-HISTORY_LIMIT), future: [] };
}

export function undo(state: OrganizerState): OrganizerState {
  const previous = state.past[state.past.length - 1];
  if (!previous) return state;
  return { items: previous, past: state.past.slice(0, -1), future: [state.items, ...state.future] };
}

export function redo(state: OrganizerState): OrganizerState {
  const next = state.future[0];
  if (!next) return state;
  return { items: next, past: [...state.past, state.items], future: state.future.slice(1) };
}

export const normalize = (deg: number) => ((deg % 360) + 360) % 360;

export function rotateItems(items: PageItem[], uids: Set<string>, delta: number): PageItem[] {
  return items.map((it) => (uids.has(it.uid) ? { ...it, rotate: normalize(it.rotate + delta) } : it));
}

export function deleteItems(items: PageItem[], uids: Set<string>): PageItem[] {
  return items.filter((it) => !uids.has(it.uid));
}

/** Inserts a copy right after each selected page. Returns the new items and the copies' ids. */
export function duplicateItems(items: PageItem[], uids: Set<string>): { items: PageItem[]; copies: string[] } {
  const copies: string[] = [];
  const out: PageItem[] = [];
  for (const it of items) {
    out.push(it);
    if (uids.has(it.uid)) {
      const copy = { ...it, uid: newUid() };
      copies.push(copy.uid);
      out.push(copy);
    }
  }
  return { items: out, copies };
}

/** Moves the given pages (keeping their relative order) to `index` in the list without them. */
export function moveItems(items: PageItem[], uids: string[], index: number): PageItem[] {
  const set = new Set(uids);
  const rest = items.filter((it) => !set.has(it.uid));
  const block = items.filter((it) => set.has(it.uid));
  const at = Math.max(0, Math.min(index, rest.length));
  return [...rest.slice(0, at), ...block, ...rest.slice(at)];
}

/** True when the list differs from the untouched document. */
export function isModified(items: PageItem[], pageCount: number): boolean {
  return items.length !== pageCount || items.some((it, i) => it.page !== i + 1 || it.rotate !== 0);
}

/** The payload for the backend's organize operation. */
export function toOperationPages(items: PageItem[]) {
  return items.map((it) => ({ page: it.page, rotate: it.rotate }));
}

/** Range selection for shift-click: ids between `from` and `to` inclusive. */
export function rangeBetween(items: PageItem[], from: string, to: string): string[] {
  const a = items.findIndex((i) => i.uid === from);
  const b = items.findIndex((i) => i.uid === to);
  if (a < 0 || b < 0) return [to];
  const [lo, hi] = a < b ? [a, b] : [b, a];
  return items.slice(lo, hi + 1).map((i) => i.uid);
}
