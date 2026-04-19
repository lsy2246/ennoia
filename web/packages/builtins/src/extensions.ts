import type {
  ExtensionPageDescriptor,
  ExtensionPanelDescriptor,
} from "@ennoia/ui-sdk";

export const builtinExtensionPages: Record<string, ExtensionPageDescriptor> = {
  "observatory.events.page": {
    mount: "observatory.events.page",
    eyebrow: "Observatory",
    summary: "观察私聊、群聊、run 与 artifact 之间的交接轨迹。",
    highlights: ["事件时间线", "线程热区", "run 交接"],
  },
  "github.repositories.page": {
    mount: "github.repositories.page",
    eyebrow: "GitHub",
    summary: "承接仓库、活动流和后续 provider 视图的正式接入位。",
    highlights: ["仓库目录", "协作活动", "provider 对接"],
  },
};

export const builtinExtensionPanels: Record<string, ExtensionPanelDescriptor> = {
  "observatory.timeline.panel": {
    mount: "observatory.timeline.panel",
    summary: "按线程聚合消息、run、task 和 artifact 的变化节奏。",
    slot: "right",
    metricLabel: "Timeline items",
  },
  "github.activity.panel": {
    mount: "github.activity.panel",
    summary: "预留仓库活动与协作状态的面板挂载入口。",
    slot: "left",
    metricLabel: "Activity items",
  },
};
