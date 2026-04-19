import { create } from "zustand";

import {
  fetchBootstrapState,
  fetchMe,
  getAuthToken,
  login as apiLogin,
  logout as apiLogout,
  setAuthToken,
  type AuthedUser,
  type BootstrapState,
} from "@ennoia/api-client";

type Status = "idle" | "checking" | "ready" | "error";

interface AuthState {
  status: Status;
  token: string | null;
  user: AuthedUser | null;
  bootstrap: BootstrapState | null;
  error: string | null;
  hydrate: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshBootstrap: () => Promise<void>;
  setUser: (user: AuthedUser | null) => void;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  status: "idle",
  token: getAuthToken(),
  user: null,
  bootstrap: null,
  error: null,

  async hydrate() {
    set({ status: "checking", error: null });
    try {
      const bootstrap = await fetchBootstrapState();
      const token = getAuthToken();
      let user: AuthedUser | null = null;
      if (token) {
        try {
          user = await fetchMe();
        } catch {
          setAuthToken(null);
        }
      } else {
        // even without token, we may be in None-mode → /auth/me still works and
        // returns anonymous
        try {
          user = await fetchMe();
        } catch {
          user = null;
        }
      }
      set({ status: "ready", token: getAuthToken(), user, bootstrap });
    } catch (error) {
      set({ status: "error", error: String(error) });
    }
  },

  async login(username, password) {
    const response = await apiLogin({ username, password });
    setAuthToken(response.token);
    set({ token: response.token });
    const user = await fetchMe();
    set({ user });
  },

  async logout() {
    try {
      await apiLogout();
    } catch {
      // ignore
    }
    setAuthToken(null);
    set({ token: null, user: null });
    await get().hydrate();
  },

  async refreshBootstrap() {
    try {
      const bootstrap = await fetchBootstrapState();
      set({ bootstrap });
    } catch {
      // ignore
    }
  },

  setUser(user) {
    set({ user });
  },
}));
