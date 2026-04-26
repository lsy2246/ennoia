# Memory

Memory 扩展负责记忆工作台、回忆、审查与上下文组装。

- 提供 `memory.*` 合同
- 消费宿主事件总线里的会话事件
- 不反向依赖 conversation 扩展实现
