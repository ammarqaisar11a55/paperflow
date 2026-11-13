import { create } from "zustand";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/services/api";
import type { ActivityEntry, JobSnapshot, PdfOperation } from "@/types";

export const JOB_EVENT = "job-update";

interface JobsState {
  jobs: Record<string, JobSnapshot>;
  activity: ActivityEntry[];
  started: boolean;
  init: () => Promise<UnlistenFn | undefined>;
  apply: (snapshot: JobSnapshot) => void;
  start: (op: PdfOperation) => Promise<string>;
  cancel: (id: string) => Promise<void>;
  clearFinished: () => Promise<void>;
  loadActivity: () => Promise<void>;
  removeActivity: (id: string) => Promise<void>;
  clearActivity: () => Promise<void>;
}

export const isFinished = (j: JobSnapshot) => j.status === "completed" || j.status === "failed" || j.status === "cancelled";

export const useJobs = create<JobsState>((set, get) => ({
  jobs: {},
  activity: [],
  started: false,
  init: async () => {
    if (get().started) return undefined;
    set({ started: true });
    const unlisten = await listen<JobSnapshot>(JOB_EVENT, (e) => get().apply(e.payload));
    try {
      const existing = await api.listJobs();
      set((s) => ({ jobs: { ...Object.fromEntries(existing.map((j) => [j.id, j])), ...s.jobs } }));
    } catch {
      /* no backend (tests) */
    }
    await get().loadActivity();
    return unlisten;
  },
  apply: (snapshot) => {
    const prev = get().jobs[snapshot.id];
    // Ignore stale events that arrive after a newer state.
    if (prev && isFinished(prev) && !isFinished(snapshot)) return;
    set((s) => ({ jobs: { ...s.jobs, [snapshot.id]: snapshot } }));
    if (isFinished(snapshot) && (!prev || !isFinished(prev))) {
      void get().loadActivity();
    }
  },
  start: async (op) => api.startJob(op),
  cancel: async (id) => {
    await api.cancelJob(id);
  },
  clearFinished: async () => {
    await api.clearFinishedJobs().catch(() => undefined);
    set((s) => ({ jobs: Object.fromEntries(Object.entries(s.jobs).filter(([, j]) => !isFinished(j))) }));
  },
  loadActivity: async () => {
    try {
      set({ activity: await api.getActivity() });
    } catch {
      /* keep the previous list */
    }
  },
  removeActivity: async (id) => {
    await api.removeActivity(id);
    set((s) => ({ activity: s.activity.filter((a) => a.id !== id) }));
  },
  clearActivity: async () => {
    await api.clearActivity();
    set({ activity: [] });
  },
}));

export function activeJobs(jobs: Record<string, JobSnapshot>): JobSnapshot[] {
  return Object.values(jobs)
    .filter((j) => !isFinished(j))
    .sort((a, b) => a.createdAt - b.createdAt);
}
