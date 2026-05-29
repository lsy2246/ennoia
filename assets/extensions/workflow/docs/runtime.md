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
- 是否从 conversation 进入 workflow，由 `conversation.operator_message.received` / `conversation.response` pipeline slot 和会话级 activation 控制。
- 任务模式开启后只影响后续消息；handler 进入 draft / plan / confirmation / execution 循环，方案不合适时继续修订同一个 draft，确认后再创建 run。
- 普通 fallback 不靠关键词或复杂度推断自动升级为任务编排。
- workflow 结果如何回写 conversation / memory，由 workflow run 状态机、系统动作管道与事件链控制。
