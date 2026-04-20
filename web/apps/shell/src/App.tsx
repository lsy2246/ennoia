import { RouterProvider } from "@tanstack/react-router";
import { useEffect } from "react";

import { router } from "@/router";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

export function App() {
  const runtimeHydrate = useRuntimeStore((state) => state.hydrate);
  const runtimeStatus = useRuntimeStore((state) => state.status);
  const uiHydrate = useUiStore((state) => state.hydrate);
  const uiStatus = useUiStore((state) => state.status);
  const { t } = useUiHelpers();

  useEffect(() => {
    runtimeHydrate();
    uiHydrate();
  }, [runtimeHydrate, uiHydrate]);

  if (
    runtimeStatus === "idle" ||
    runtimeStatus === "checking" ||
    uiStatus === "idle" ||
    uiStatus === "checking"
  ) {
    return (
      <div className="page page--centered">
        <p>{t("shell.loading.connecting", "Connecting to Ennoia…")}</p>
      </div>
    );
  }

  return <RouterProvider router={router} />;
}
