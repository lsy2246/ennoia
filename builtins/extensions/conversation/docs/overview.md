# Conversation

Conversation 扩展负责提供会话、分支、检查点、消息与 lane 的系统级接口。

- 通过 `conversation.*`、`branch.*`、`checkpoint.*`、`message.*`、`lane.*` 合同暴露稳定能力
- 负责会话主数据的持久化与读取
- 负责会话分支、改写重发与上下文重开
- 不直接承担记忆与编排逻辑
