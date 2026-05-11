# 变更提案: assets-root-reorg

## 元信息
```yaml
类型: 重构
方案类型: implementation
优先级: P1
状态: 已确认
创建: 2026-05-11
```

---

## 1. 需求

### 背景
当前仓库把内置扩展与技能源码放在 `builtins/`，把运行目录初始化模板放在
`assets/templates/config/`。这导致仓库根目录的边界表达不统一：

- `builtins` 更像内容归属而不是稳定的源码边界名
- `assets/templates/config` 层级过深，`templates` 下几乎只有 `config`
- CLI、脚本、前端配置和文档中都直接硬编码了 `builtins/...` 路径，后续维护成本高

### 目标
- 把官方内置扩展源码统一迁移到 `assets/extensions/*`
- 把官方内置技能源码统一迁移到 `assets/skills/*`
- 把 `assets/templates/config/*.toml` 压平为 `assets/templates/*.toml`
- 同步更新 Rust、脚本、前端配置、测试与文档中的路径引用
- 保持运行时语义不变，不触及 `packaging/`

### 约束条件
```yaml
时间约束: 当前回合内完成目录重组和基础校验
性能约束: 不改变运行时加载、内置物化和 watcher 行为
兼容性约束: 允许源码路径变更，不需要维护旧目录兼容层
业务约束: 本次不讨论也不修改 packaging 目录职责
```

### 验收标准
- [x] 仓库不再存在 `builtins/` 顶层源码根，内置源码统一收敛到 `assets/`
- [x] `assets/templates/server.toml` 与 `assets/templates/ui.toml` 成为唯一模板路径
- [x] CLI、构建脚本、前端配置、测试和文档已切换到新路径
- [x] `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace` 通过
- [x] 已补充执行 `web lint`、`web typecheck`、`web build`、`build-extension-ui` 与 `build-workers`

---

## 2. 方案

### 技术方案
本次采用“目录先收敛、语义不变”的重构策略：

1. 直接移动源码目录：
   - `builtins/extensions/*` → `assets/extensions/*`
   - `builtins/skills/*` → `assets/skills/*`
   - `assets/templates/config/*.toml` → `assets/templates/*.toml`
2. 修改 `crates/assets` 的编译期资源收集逻辑，让 builtin 资源从新的
   `assets/extensions` 与 `assets/skills` 汇总，而模板从压平后的
   `assets/templates` 读取
3. 修改 CLI、worker/UI 构建脚本、前端配置和测试中的硬编码路径
4. 更新仓库文档，把“内置能力源码根”统一表述为 `assets/`

### 影响范围
```yaml
涉及模块:
  - crates/assets: 编译期资源收集根路径调整
  - crates/cli: dev watcher、builtin 发现、测试路径调整
  - scripts: worker 与 extension UI 构建路径调整
  - web: tsconfig 与 eslint 对扩展 UI 源码的引用路径调整
  - docs: 内置源码位置与开发链路说明调整
预计变更文件: 15-25
```

### 风险评估
| 风险 | 等级 | 应对 |
|------|------|------|
| 构建脚本遗漏旧路径 | 中 | 全仓搜索 `builtins/` 和旧模板路径后逐项替换 |
| CLI watcher 或测试仍依赖旧目录 | 中 | 修改后执行 `cargo test --workspace` 覆盖回归 |
| 文档与代码路径不一致 | 低 | 本次和代码一起同步更新 README 与协议文档 |

---

## 3. 技术设计（可选）

### 路径映射

```text
builtins/extensions/<id>/...      -> assets/extensions/<id>/...
builtins/skills/<id>/...          -> assets/skills/<id>/...
assets/templates/config/server.toml -> assets/templates/server.toml
assets/templates/config/ui.toml     -> assets/templates/ui.toml
```

### 资源收集策略

- `templates`：继续作为运行目录初始化模板集合，由 `crates/assets` 直接读取
- `builtins` 逻辑集合：不再对应仓库顶层目录，而是由
  `assets/extensions/*` 与 `assets/skills/*` 组合生成
- 运行时物化结果保持不变，仍写入 `<ENNOIA_HOME>/extensions/*` 与
  `<ENNOIA_HOME>/skills/*`

---

## 4. 核心场景

> 执行完成后同步到对应模块文档

### 场景: 初始化运行目录
**模块**: `crates/assets` + `crates/cli`
**条件**: 用户执行 `ennoia init` 或 `ennoia dev`
**行为**: CLI 从 `assets/templates/*.toml` 读取模板，并从 `assets/extensions/*`、
`assets/skills/*` 读取内置包内容后物化到 home 目录
**结果**: 运行目录结构与现有行为保持一致，只是源码根路径完成重组

### 场景: 开发态扩展 UI 与 worker 热重载
**模块**: `scripts/` + `crates/cli` + `web`
**条件**: 用户执行 `npm run dev`
**行为**: watcher 和构建脚本从 `assets/extensions/*` 发现扩展 UI、worker 与数据目录
**结果**: 开发链路继续可用，不再依赖 `builtins/` 顶层路径

---

## 5. 技术决策

> 本方案涉及的技术决策，归档后成为决策的唯一完整记录

### assets-root-reorg#D001: 内置源码根并入 assets
**日期**: 2026-05-11
**状态**: ✅采纳
**背景**: 现有 `builtins/` 顶层目录与 `assets/` 平级，导致仓库边界表达分裂，且
`assets/templates/config` 层级冗余。
**选项分析**:
| 选项 | 优点 | 缺点 |
|------|------|------|
| A: `assets/extensions` + `assets/skills` + `assets/templates` | 目录语义统一，路径更短，符合当前目标 | 需要批量修改硬编码路径 |
| B: 顶层改为 `extensions` + `skills`，`assets` 只留模板 | 语义也清晰 | 偏离本轮已确认方向 |
**决策**: 选择方案 A
**理由**: 最符合当前需求收敛结果，也能在不引入额外抽象层的前提下完成整理。
**影响**: 影响 CLI、脚本、前端配置、测试与文档中的源码路径引用

---

## 6. 成果设计

> 含视觉产出的任务由 DESIGN Phase2 填充。非视觉任务整节标注"N/A"。

N/A
