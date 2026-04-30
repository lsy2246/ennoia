# Workflow Runtime

- `run.create`
- `run.get`
- `run.list`
- `task.list`
- `artifact.list`
- `workflow.run`

说明：

- `workflow` 自身只拥有 run/task/artifact 事实。
- `conversation_id` 只是可选引用字段，不代表 workflow 必须依赖 conversation。
- 是否从 conversation 自动触发 run，或把 workflow 结果再写回 conversation / memory，由系统动作管道与事件链控制。
