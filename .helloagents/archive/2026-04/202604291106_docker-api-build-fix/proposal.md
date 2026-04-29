# 变更提案: docker-api-build-fix

## 元信息
```yaml
类型: 修复
方案类型: implementation
优先级: P0
状态: 已确认
创建: 2026-04-29
```

---

## 1. 需求

### 背景
执行 `docker compose build` 时，`api` 阶段在 `cargo build --release --bin ennoia` 失败。复现日志显示容器内缺少 `builtins/extensions/*` 这些 workspace 成员目录，导致 Cargo 无法读取对应 `Cargo.toml`。

### 目标
- 修复 Docker API 镜像构建失败。
- 保持改动最小，不改变现有 Rust workspace 结构和二进制入口。
- 在修复后重新验证 `api` 目标能够成功完成构建。

### 约束条件
```yaml
时间约束: 以最小范围修复当前构建阻塞问题
性能约束: 不引入额外运行时开销
兼容性约束: 保持现有 Docker 多阶段构建和 workspace 布局不变
业务约束: 不影响 web 镜像构建逻辑
```

### 验收标准
- [ ] `docker build --target api -f Dockerfile .` 可以成功完成
- [ ] 修复后的 Dockerfile 能覆盖根 `Cargo.toml` 中参与构建解析的 workspace 成员目录
- [ ] 不修改无关业务代码和已有 workspace 配置

---

## 2. 方案

### 技术方案
在 `Dockerfile` 的 `api-build` 阶段补充 `COPY builtins ./builtins`，让容器中的 `/app` 拥有根 workspace 中声明的所有内置扩展 crate。随后重新执行 API 目标构建验证问题是否消失。

### 影响范围
```yaml
涉及模块:
  - docker-build: API 镜像构建上下文补全
  - rust-workspace: 容器内 Cargo workspace 解析恢复正常
预计变更文件: 1
```

### 风险评估
| 风险 | 等级 | 应对 |
|------|------|------|
| `builtins/` 目录加入构建上下文后镜像构建时间略有增加 | 低 | 保持只复制必要目录，后续若需要可再做分层优化 |
| 仅修复缺失目录后仍存在其他 Rust 编译问题 | 低 | 通过重新执行 Docker 构建继续验证完整结果 |

---

## 3. 技术设计（可选）

本次为构建修复，不涉及架构、API 或数据模型变更。

### 架构设计
N/A

### API设计
N/A

### 数据模型
N/A

---

## 4. 核心场景

> 执行完成后同步到对应模块文档

### 场景: Docker API 构建
**模块**: docker-build
**条件**: 在仓库根目录执行 `docker build --target api -f Dockerfile .`
**行为**: Docker 将 Cargo workspace 所需目录复制进 `api-build` 阶段，再执行 `cargo build --release --bin ennoia`
**结果**: Cargo 成功解析 workspace 成员并输出 `ennoia` 可执行文件

---

## 5. 技术决策

> 本方案涉及的技术决策，归档后成为决策的唯一完整记录

### docker-api-build-fix#D001: 在 Docker API 构建阶段复制 `builtins/`
**日期**: 2026-04-29
**状态**: ✅采纳
**背景**: 根 `Cargo.toml` 把 `builtins/extensions/*` 下的多个 crate 声明为 workspace 成员，但现有 `Dockerfile` 只复制了 `Cargo.toml`、`Cargo.lock`、`assets/` 和 `crates/`，容器内缺少这些成员目录，导致构建提前失败。
**选项分析**:
| 选项 | 优点 | 缺点 |
|------|------|------|
| A: 在 Dockerfile 中补充复制 `builtins/` | 修改最小，和现有 workspace 结构保持一致，能直接修复缺失目录问题 | 构建上下文会略增大 |
| B: 改写 workspace 或为 Docker 单独维护精简 manifest | 可能进一步减小镜像构建输入 | 改动面更大，容易引入新的配置分叉 |
**决策**: 选择方案 A
**理由**: 当前失败原因是容器内缺失 workspace 成员目录，直接补齐构建输入是最低风险、最快见效的修复。
**影响**: 影响 `Dockerfile` 的 `api-build` 阶段，不影响运行时行为

---

## 6. 成果设计

> 含视觉产出的任务由 DESIGN Phase2 填充。非视觉任务整节标注"N/A"。

### 设计方向
- **美学基调**: N/A
- **记忆点**: N/A
- **参考**: 无

### 视觉要素
- **配色**: N/A
- **字体**: N/A
- **布局**: N/A
- **动效**: N/A
- **氛围**: N/A

### 技术约束
- **可访问性**: N/A
- **响应式**: N/A
