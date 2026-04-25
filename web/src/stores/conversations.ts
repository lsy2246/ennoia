import { create } from "zustand";

type ConversationsState = {
  revision: number;
  deletedSessionMarks: Record<string, number>;
  notifyChanged: () => void;
  notifyDeleted: (sessionId: string) => void;
};

export const useConversationsStore = create<ConversationsState>((set) => ({
  revision: 0,
  deletedSessionMarks: {},
  notifyChanged() {
    set((state) => ({ revision: state.revision + 1 }));
  },
  notifyDeleted(sessionId) {
    set((state) => ({
      revision: state.revision + 1,
      deletedSessionMarks: {
        ...state.deletedSessionMarks,
        [sessionId]: Date.now(),
      },
    }));
  },
}));
