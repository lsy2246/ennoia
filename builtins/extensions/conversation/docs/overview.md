# Conversation

Conversation 扩展负责提供会话、消息与 lane 的系统级接口。

- 通过 `conversation.*`、`message.*`、`lane.*` 合同暴露稳定能力
- 负责会话主数据的持久化与读取
- 不直接承担记忆与编排逻辑
