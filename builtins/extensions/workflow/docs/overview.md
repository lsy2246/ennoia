# Workflow

Workflow 扩展负责 run、task、artifact 与调度动作。

- 暴露 `run.*`、`task.*`、`artifact.*` 等合同
- 作为系统编排运行时的默认实现
- 供 scheduler 通过 `workflow.run` 动作触发
