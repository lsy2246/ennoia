# Workflow

Workflow 扩展负责 run、task、artifact 与调度动作。

- 暴露 `run.*`、`task.*`、`artifact.*` 等合同
- 作为系统编排运行时的默认实现
- 通过 `conversation.response` pipeline slot 提供会话级处理策略，策略选择只影响后续消息
- 供 scheduler 通过 `workflow.run` 动作触发
- 扩展通过会话内通用 record 渲染，在确实产生 plan/run 时展示执行轨迹卡，作为主要查看入口
- 自带 Workflow Studio 页面，作为辅助入口查看跨会话 run 的阶段流转、gate、task 与 artifact
- 是否与会话、记忆自动联动，由 pipeline activation、系统动作管道与事件链决定
