import { create } from "zustand";

import {
  fetchBootstrapStatus,
  fetchRuntimeProfile,
  type BootstrapState,
  type WorkspaceProfile,
} from "@ennoia/api-client";

type RuntimeStatus = "idle" | "checking" | "ready" | "error";

type RuntimeState = {
  status: RuntimeStatus;
  bootstrap: BootstrapState | null;
  profile: WorkspaceProfile | null;
  error: string | null;
  hydrate: () => Promise<void>;
};

export const useRuntimeStore = create<RuntimeState>((set) => ({
  status: "idle",
  bootstrap: null,
  profile: null,
  error: null,

  async hydrate() {
    set({ status: "checking", error: null });
    try {
      const bootstrap = await fetchBootstrapStatus();
      const profile = bootstrap.is_initialized ? await fetchRuntimeProfile() : null;
      set({
        status: "ready",
        bootstrap,
        profile,
      });
    } catch (error) {
      set({
        status: "error",
        error: String(error),
      });
    }
  },
}));
