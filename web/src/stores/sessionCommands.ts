import { create } from "zustand";

import type { ChatBranch, ChatCheckpoint } from "@ennoia/api-client";

export type SessionCommandRegistration = {
  panelId: string;
  sessionId: string;
  title: string;
  activeBranchId?: string | null;
  branches: ChatBranch[];
  checkpoints: ChatCheckpoint[];
  actions: {
    resetContext: () => void;
    createCheckpoint: () => void;
    switchBranch: (branchId: string) => void;
    branchFromCheckpoint: (checkpointId: string) => void;
  };
};

type SessionCommandState = {
  items: Record<string, SessionCommandRegistration>;
  register: (registration: SessionCommandRegistration) => void;
  unregister: (panelId: string) => void;
};

export const useSessionCommandsStore = create<SessionCommandState>((set) => ({
  items: {},
  register(registration) {
    set((state) => ({
      items: {
        ...state.items,
        [registration.panelId]: registration,
      },
    }));
  },
  unregister(panelId) {
    set((state) => {
      const next = { ...state.items };
      delete next[panelId];
      return { items: next };
    });
  },
}));
