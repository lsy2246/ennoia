import React from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ExtensionUiModule } from "@ennoia/ui-sdk";

import MemoryExtensionPage from "./page/Page";

const roots = new WeakMap<HTMLElement, Root>();

const extensionUi: ExtensionUiModule = {
  pages: {
    "memory.page": (container, context) => {
      let root = roots.get(container);
      if (!root) {
        root = createRoot(container);
        roots.set(container, root);
      }
      root.render(
        <React.StrictMode>
          <MemoryExtensionPage helpers={context.helpers} />
        </React.StrictMode>,
      );
      return {
        unmount() {
          const current = roots.get(container);
          current?.unmount();
          roots.delete(container);
        },
      };
    },
  },
};

export default extensionUi;
