import { describe, expect, it } from "vitest";
import {
  basename,
  describePages,
  dirname,
  expandRanges,
  formatBytes,
  pluralize,
  prettyPath,
  reduction,
  relativeTime,
  stem,
  validateRanges,
} from "./format";

describe("formatBytes", () => {
  it("formats sizes for people", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(25 * 1024 * 1024)).toBe("25 MB");
    expect(formatBytes(-1)).toBe("—");
  });
});

describe("reduction", () => {
  it("reports the percentage saved", () => {
    expect(reduction(1000, 400)).toBe(60);
    expect(reduction(1000, 1200)).toBe(-20);
    expect(reduction(0, 10)).toBe(0);
  });
});

describe("paths", () => {
  it("splits names", () => {
    expect(basename("/a/b/Report.pdf")).toBe("Report.pdf");
    expect(dirname("/a/b/Report.pdf")).toBe("/a/b");
    expect(stem("/a/b/Report.final.pdf")).toBe("Report.final");
    expect(prettyPath("/home/ada/Documents/x.pdf")).toBe("~/Documents/x.pdf");
    expect(prettyPath("/mnt/disk/x.pdf")).toBe("/mnt/disk/x.pdf");
  });
});

describe("page ranges", () => {
  it("accepts the same syntax as the backend", () => {
    expect(validateRanges("1-3, 5, 8-", 10)).toBeNull();
    expect(validateRanges("-2; end", 10)).toBeNull();
    expect(validateRanges("", 10)).toBeNull();
    expect(validateRanges("", 10, false)).not.toBeNull();
  });

  it("explains mistakes", () => {
    expect(validateRanges("0", 10)).toMatch(/start at 1/);
    expect(validateRanges("11", 10)).toMatch(/10 pages/);
    expect(validateRanges("a-b", 10)).toMatch(/not a valid/);
  });

  it("expands and describes pages", () => {
    expect(expandRanges("1-3, 2, 9-", 10)).toEqual([1, 2, 3, 9, 10]);
    expect(expandRanges("3-1", 5)).toEqual([3, 2, 1]);
    expect(expandRanges("", 3)).toEqual([1, 2, 3]);
    expect(describePages([1, 2, 3, 5, 7, 8])).toBe("1-3, 5, 7-8");
  });
});

describe("text", () => {
  it("pluralizes", () => {
    expect(pluralize(1, "page")).toBe("1 page");
    expect(pluralize(3, "page")).toBe("3 pages");
  });

  it("describes times relative to now", () => {
    const now = Date.UTC(2026, 10, 13, 12);
    expect(relativeTime(now - 10_000, now)).toBe("Just now");
    expect(relativeTime(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
    expect(relativeTime(now - 26 * 3_600_000, now)).toBe("Yesterday");
  });
});
