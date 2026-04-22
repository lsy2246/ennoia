import type { ComponentType } from "react";

import { MemoryExtensionPage } from "../../../../builtins/extensions/memory/ui/page/Page";

export const extensionPageComponents: Record<string, ComponentType> = {
  "memory.page": MemoryExtensionPage,
};
