# Conversation

Conversation 扩展负责提供原生会话事实的统一接口实现。

- 通过 `conversation.*`、`branch.*`、`message.*`、`lane.*` 合同暴露稳定能力
- 负责会话主数据的持久化与读取
- 负责会话分支与改写重发
- 不直接承担记忆与编排逻辑
