import { useCallback, useState } from "react";
import { api } from "@/services/api";
import type { FileInfo } from "@/types";
import { KINDS, type InputKind } from "@/features/fileKinds";
import { moveIds } from "./usePointerSort";

export interface SelectionOptions {
  accepts: InputKind;
  /** Keep at most one file (a new one replaces it). */
  single?: boolean;
}

/**
 * The files chosen for a tool: inspected by the backend (type, pages,
 * protection), deduplicated, and in the user's order.
 */
export function useFileSelection({ accepts, single }: SelectionOptions) {
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [rejected, setRejected] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  const add = useCallback(
    async (paths: string[]) => {
      if (!paths.length) return;
      setLoading(true);
      try {
        const infos = await api.inspectFiles(paths);
        const ok = infos.filter((f) => f.kind === accepts && !f.error);
        const bad = infos.filter((f) => !(f.kind === accepts && !f.error));
        setRejected(
          bad.map((f) =>
            f.kind !== accepts
              ? `“${f.name}” is not ${KINDS[accepts].one}.`
              : (f.error ?? `“${f.name}” could not be read.`),
          ),
        );
        setFiles((current) => {
          if (single) return ok.length ? [ok[0]] : current;
          const known = new Set(current.map((f) => f.path));
          return [...current, ...ok.filter((f) => !known.has(f.path) && known.add(f.path))];
        });
      } catch (e) {
        setRejected([(e as { message?: string }).message ?? "The files could not be read."]);
      } finally {
        setLoading(false);
      }
    },
    [accepts, single],
  );

  const remove = useCallback((path: string) => setFiles((fs) => fs.filter((f) => f.path !== path)), []);
  const clear = useCallback(() => {
    setFiles([]);
    setRejected([]);
  }, []);
  const move = useCallback((path: string, delta: number) => {
    setFiles((fs) => {
      const i = fs.findIndex((f) => f.path === path);
      const j = i + delta;
      if (i < 0 || j < 0 || j >= fs.length) return fs;
      const next = [...fs];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
  }, []);
  const reorder = useCallback((moving: string[], index: number) => {
    setFiles((fs) => {
      const order = moveIds(
        fs.map((f) => f.path),
        moving,
        index,
      );
      const byPath = new Map(fs.map((f) => [f.path, f]));
      return order.map((p) => byPath.get(p)!);
    });
  }, []);
  const sortByName = useCallback(
    () => setFiles((fs) => [...fs].sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }))),
    [],
  );

  return { files, rejected, loading, add, remove, clear, move, reorder, sortByName, dismissRejected: () => setRejected([]) };
}

export type FileSelection = ReturnType<typeof useFileSelection>;
