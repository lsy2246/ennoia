import { RouterProvider } from "@tanstack/react-router";
import { useEffect } from "react";

import { router } from "@/router";
import { useAuthStore } from "@/stores/auth";

export function App() {
  const hydrate = useAuthStore((s) => s.hydrate);
  const status = useAuthStore((s) => s.status);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  if (status === "idle" || status === "checking") {
    return (
      <div className="page page--centered">
        <p>Connecting to Ennoia…</p>
      </div>
    );
  }

  return <RouterProvider router={router} />;
}
