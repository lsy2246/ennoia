import React from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ExtensionUiModule } from "@ennoia/ui-sdk";

import MemoryExtensionPage from "./page/Page";

const roots = new WeakMap<HTMLElement, Root>();

function MemoryAboutPage() {
  return (
    <div className="resource-layout" style={{ gridTemplateColumns: "minmax(0, 1fr)" }}>
      <section className="work-panel">
        <div className="page-heading">
          <span>Memory</span>
          <h1>关于记忆</h1>
          <p>记忆扩展负责长期记忆、回忆、审查和上下文装配；会话事实本身仍由系统会话模块维护。</p>
        </div>
        <div className="card-grid">
          {[
            ["职责", "聚焦 recall、review、context assembly，不镜像承载整段原生会话消息。"],
            ["入口", "主入口是记忆工作台；这个关于页用来解释边界、入口和接入方式。"],
            ["接入", "只有显式声明 conversation 规则时，系统才会把记忆能力整理进会话目录。"],
          ].map(([title, body]) => (
            <article key={title} className="mini-card">
              <strong>{title}</strong>
              <p>{body}</p>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

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
    "memory.about": (container) => {
      let root = roots.get(container);
      if (!root) {
        root = createRoot(container);
        roots.set(container, root);
      }
      root.render(
        <React.StrictMode>
          <MemoryAboutPage />
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
