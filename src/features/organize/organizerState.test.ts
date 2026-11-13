import { describe, expect, it } from "vitest";
import {
  commit,
  deleteItems,
  duplicateItems,
  initialState,
  isModified,
  moveItems,
  rangeBetween,
  redo,
  rotateItems,
  toOperationPages,
  undo,
} from "./organizerState";

const pages = (items: { page: number }[]) => items.map((i) => i.page);

describe("page organizer state", () => {
  it("starts as the untouched document", () => {
    const s = initialState(4);
    expect(pages(s.items)).toEqual([1, 2, 3, 4]);
    expect(isModified(s.items, 4)).toBe(false);
  });

  it("rotates, deletes, duplicates and moves pages", () => {
    let { items } = initialState(4);
    const [a, b, c, d] = items.map((i) => i.uid);

    items = rotateItems(items, new Set([b]), 90);
    items = rotateItems(items, new Set([b]), 270);
    expect(items[1].rotate).toBe(0);
    items = rotateItems(items, new Set([c]), -90);
    expect(items[2].rotate).toBe(270);

    const dup = duplicateItems(items, new Set([a]));
    expect(pages(dup.items)).toEqual([1, 1, 2, 3, 4]);
    expect(dup.copies).toHaveLength(1);

    items = deleteItems(dup.items, new Set([b]));
    expect(pages(items)).toEqual([1, 1, 3, 4]);

    items = moveItems(items, [d], 0);
    expect(pages(items)).toEqual([4, 1, 1, 3]);
    expect(isModified(items, 4)).toBe(true);
    expect(toOperationPages(items)).toEqual([
      { page: 4, rotate: 0 },
      { page: 1, rotate: 0 },
      { page: 1, rotate: 0 },
      { page: 3, rotate: 270 },
    ]);
  });

  it("moves a selection as a block, keeping its order", () => {
    const { items } = initialState(6);
    const ids = items.map((i) => i.uid);
    expect(pages(moveItems(items, [ids[4], ids[1]], 0))).toEqual([2, 5, 1, 3, 4, 6]);
    expect(pages(moveItems(items, [ids[0]], 99))).toEqual([2, 3, 4, 5, 6, 1]);
  });

  it("undoes and redoes every change", () => {
    let s = initialState(3);
    const original = s.items;
    s = commit(s, deleteItems(s.items, new Set([s.items[0].uid])));
    s = commit(s, rotateItems(s.items, new Set([s.items[0].uid]), 90));
    expect(pages(s.items)).toEqual([2, 3]);
    s = undo(s);
    expect(s.items[0].rotate).toBe(0);
    s = undo(s);
    expect(s.items).toBe(original);
    expect(undo(s)).toBe(s);
    s = redo(redo(s));
    expect(s.items[0].rotate).toBe(90);
    expect(redo(s)).toBe(s);
  });

  it("a new change clears the redo history", () => {
    let s = initialState(2);
    s = commit(s, rotateItems(s.items, new Set([s.items[0].uid]), 90));
    s = undo(s);
    s = commit(s, deleteItems(s.items, new Set([s.items[1].uid])));
    expect(s.future).toHaveLength(0);
  });

  it("selects ranges in either direction", () => {
    const { items } = initialState(5);
    const ids = items.map((i) => i.uid);
    expect(rangeBetween(items, ids[3], ids[1])).toEqual(ids.slice(1, 4));
    expect(rangeBetween(items, ids[0], ids[0])).toEqual([ids[0]]);
  });
});
