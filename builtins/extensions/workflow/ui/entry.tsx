import React from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ExtensionUiModule } from "@ennoia/ui-sdk";

import WorkflowConversationRecord from "./conversation/RecordCard";
import WorkflowPage from "./page/Page";

const roots = new WeakMap<HTMLElement, Root>();

function renderIntoContainer(
  container: HTMLElement,
  node: React.ReactNode,
) {
  let root = roots.get(container);
  if (!root) {
    root = createRoot(container);
    roots.set(container, root);
  }
  root.render(<React.StrictMode>{node}</React.StrictMode>);
  return {
    unmount() {
      const current = roots.get(container);
      current?.unmount();
      roots.delete(container);
    },
  };
}

const extensionUi: ExtensionUiModule = {
  pages: {
    "workflow.page": (container, context) =>
      renderIntoContainer(
        container,
        <WorkflowPage helpers={context.helpers} />,
      ),
  },
  conversationRecords: {
    default: (container, context) =>
      renderIntoContainer(
        container,
        <WorkflowConversationRecord
          record={context.record}
          helpers={context.helpers}
        />,
      ),
  },
};

export default extensionUi;
