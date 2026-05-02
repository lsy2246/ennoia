import { create } from "zustand";

type AgentsState = {
  revision: number;
  notifyChanged: () => void;
};

export const useAgentsStore = create<AgentsState>((set) => ({
  revision: 0,
  notifyChanged() {
    set((state) => ({ revision: state.revision + 1 }));
  },
}));
