// Files dropped onto the window arrive as paths through Tauri's drag-and-drop
// event. The screen currently shown registers a handler; the most recently
// registered one receives the files.
import { create } from "zustand";

type DropHandler = (paths: string[]) => void;

interface DropState {
  dragging: boolean;
  handlers: { id: number; handler: DropHandler; label: string }[];
  setDragging: (dragging: boolean) => void;
  register: (handler: DropHandler, label: string) => () => void;
  dispatch: (paths: string[]) => void;
}

let nextId = 1;

export const useDrop = create<DropState>((set, get) => ({
  dragging: false,
  handlers: [],
  setDragging: (dragging) => set({ dragging }),
  register: (handler, label) => {
    const id = nextId++;
    set((s) => ({ handlers: [...s.handlers, { id, handler, label }] }));
    return () => set((s) => ({ handlers: s.handlers.filter((h) => h.id !== id) }));
  },
  dispatch: (paths) => {
    const top = get().handlers[get().handlers.length - 1];
    set({ dragging: false });
    if (top && paths.length) top.handler(paths);
  },
}));
