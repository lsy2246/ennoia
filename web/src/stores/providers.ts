import { create } from "zustand";

type ProvidersState = {
  revision: number;
  notifyChanged: () => void;
};

export const useProvidersStore = create<ProvidersState>((set) => ({
  revision: 0,
  notifyChanged() {
    set((state) => ({ revision: state.revision + 1 }));
  },
}));
