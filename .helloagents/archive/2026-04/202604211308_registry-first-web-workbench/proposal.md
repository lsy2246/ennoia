# 变更提案: registry-first-web-workbench

## 元信息
```yaml
类型: 重构
方案类型: implementation
优先级: P0
状态: 已确认
创建: 2026-04-21
```

---

## 1. 需求

### 背景
在完成第一轮工作台重构后，用户进一步指出核心结构仍然不够合理：

- 新页面没有完整接入多语言体系
- 导航和视图仍是静态写死，不能像 VSCode 一样动态扩展
- 对外产品术语仍混用 `Web` / `网页端` / `Web`
- AI 上游的展示层级不对，用户应该选择“已实现的上游类型”，而不是看具体实现细节
- Agent 中的 Provider 语义不清，本质应是选择“上游接口”
- 任务页中的 `schedule_kind / schedule_value / run_at` 是底层字段，不是产品语言
- 长页面的滚动区域设计错误
- 现有 IDE 视觉没有真正落地拖拽停靠布局
- 工作区位置缺少统一根设计
- 时区输入在设置页仍是文本，不是下拉
- `Observatory` 底层能力已经存在，但没有作为正式 Web 视图接入

### 目标
- 把前端正式收敛为 `Web` 工作台术语
- 把一级导航、扩展视图、Inspector 和底部面板改成注册表驱动
- 给新页面全部接入 i18n
- 把拖拽停靠布局真正接入，而不是只保留 IDE 风格皮肤
- 把工作区根路径收敛为单一配置源，Agent/任务/会话基于该根派生
- 把 AI 上游设计为“已实现接口类型”，并允许扩展提供实现
- 把任务调度页重写为用户可理解的表单语言
- 把 Observatory 提升为正式可见页面/面板

### 约束条件
```yaml
兼容性: 不要求兼容第一轮刚完成的前端信息架构
技术约束: 保持本地单机模式，不引入外部服务依赖
交互约束: 优先桌面工作台体验，支持拖拽停靠和分区滚动
产品约束: 技能与扩展仍然严格分离；上游实现视为扩展贡献的一种
```

### 验收标准
- [x] 新增页面和主壳文案全部接入 i18n，默认中文/英文可切换
- [x] 导航栏支持“内建视图 + 扩展视图”动态合并展示
- [x] 对外术语统一为 `Web`
- [x] Agent 编辑中不再暴露“Provider 实现细节”，只选择上游类型
- [x] 上游实现来源移入扩展语义，不在管理页直接暴露实现来源
- [x] 任务页将 `schedule_kind / schedule_value / run_at` 重命名为用户可理解表单项
- [x] 工作台长页面可在正确分区内滚动
- [x] 主工作区支持拖拽停靠布局
- [x] 工作区根路径只有一个配置入口，其他路径基于其派生
- [x] 时区输入改为下拉
- [x] Observatory 成为正式可见的 Web 页面/面板

---

## 2. 方案

### 技术方案
采用 `Registry-First Web Workbench` 方案：

1. **命名与国际化收敛**
   - 前端对外术语统一为 `Web`
   - 新页面文案全部改为 i18n key + fallback 模式
   - 补全新的 `web.*` 或兼容映射文案组

2. **前端注册表驱动壳层**
   - 主导航由“内建导航项 + extension pages contribution”组成
   - 侧栏、Inspector、底部面板由内建卡片与扩展面板贡献合并渲染
   - Observatory 作为内建/扩展混合可见能力正式接入

3. **拖拽停靠与滚动重构**
   - 接入 `dockview` 承载编辑区/Inspector/Bottom Panel
   - 修正 CSS overflow 策略，改成区域滚动

4. **模型边界重塑**
   - Agent 将 `provider_id` 产品化为“上游类型/接口选择”
   - Provider 管理页改造成“已实现上游类型”选择与启用状态视图
   - 工作区改成全局 `workspace_root`，Agent 仅持有相对派生信息或使用默认派生

5. **设置与任务表单重构**
   - 时区统一改用下拉
   - 调度表单字段改为 `触发方式 / 规则参数 / 首次运行时间`
   - 增加字段帮助文案与示例

### 影响范围
```yaml
涉及模块:
  - web: AppWeb、router、所有主要页面、布局样式、dockview 集成
  - web/packages/api-client: 类型命名与任务表单字段适配
  - web/packages/i18n: 新增 Web 工作台词条
  - crates/kernel: 工作区根路径/上游接口字段语义调整（如需要）
  - crates/server: 设置、Provider、Workspace、扩展视图聚合接口补齐（如需要）
  - docs: README、architecture、data-model、api-surface
```

### 风险评估
| 风险 | 等级 | 应对 |
|------|------|------|
| `dockview` 接入会影响现有布局与滚动 | 高 | 先重构壳层，再逐步把页面挂入 dock 布局 |
| i18n 全量补录容易遗漏 | 中 | 统一按页面逐项替换并用搜索校验硬编码 |
| Provider/Workspace 模型重新定义会涉及前后端同步 | 高 | 先以 UI 语义收敛，再最小调整内核字段 |
| 扩展导航动态化需要兼顾现有 router 结构 | 中 | 使用“固定动态承载页 + registry 渲染”的方式避免完全动态路由 |

---

## 3. 技术设计

### 架构设计
```text
Web Workbench
  -> Builtin Navigation Registry
  -> Extension Navigation Registry
  -> Dock Layout (dockview)
      -> Primary Editor Views
      -> Secondary Inspector Panels
      -> Bottom Utility Panels
  -> API Client
      -> Server Aggregation APIs
```

### 核心设计点
- `Web` 是对外产品名称，`web` 仅保留为内部遗留目录名
- 导航不再硬编码为单一数组，而是由 registry 生成
- 扩展提供的 page/panel 可以挂接到 Web 的导航和 dock 区域
- Observatory 默认作为一个内建监控视图，并可消费扩展事件/面板贡献
- 工作区根路径统一由设置页管理，Agent 的工作路径由根路径推导

### 技术决策
#### registry-first-web-workbench#D001: 采用注册表驱动导航与视图
- 理由: 只有这样才能真正支持“像 VSCode 一样扩展视图与导航”

#### registry-first-web-workbench#D002: 使用 dockview 落地可拖拽停靠布局
- 理由: 当前依赖已存在，适合作为正式布局内核

#### registry-first-web-workbench#D003: 工作区根路径统一上提
- 理由: 用户要求工作区只在一个地方配置，其他位置基于其派生
