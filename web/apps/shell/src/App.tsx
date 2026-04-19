import { RouterProvider } from "@tanstack/react-router";
import { useEffect } from "react";

import { router } from "@/router";
import { useAuthStore } from "@/stores/auth";
import { useUiStore } from "@/stores/ui";

export function App() {
  const hydrate = useAuthStore((s) => s.hydrate);
  const authStatus = useAuthStore((s) => s.status);
  const authToken = useAuthStore((s) => s.token);
  const uiHydrate = useUiStore((s) => s.hydrate);
  const uiStatus = useUiStore((s) => s.status);

  useEffect(() => {
    hydrate();
    uiHydrate();
  }, [hydrate, uiHydrate]);

  useEffect(() => {
    if (authStatus === "ready") {
      uiHydrate();
    }
  }, [authStatus, authToken, uiHydrate]);

  if (
    authStatus === "idle" ||
    authStatus === "checking" ||
    uiStatus === "idle" ||
    uiStatus === "checking"
  ) {
    return (
      <div className="page page--centered">
        <p>Connecting to Ennoia…</p>
      </div>
    );
  }

  return <RouterProvider router={router} />;
}
