# Ennoia 前端主题协议

## 目标

`ennoia.theme` 定义 Web 主壳与扩展主题之间的稳定样式协议。
主题可以改变系统观感，但不能绕过语义变量直接耦合主壳内部 class 结构。

## 协议标识

- 当前标识：`ennoia.theme`
- 扩展主题在 `contributes.themes[]` 中可选声明 `contract = "ennoia.theme"`
- 未声明时默认按 `ennoia.theme` 解释

## 必选 Token

扩展主题至少应提供以下基础 token：

- `--color-bg`
- `--color-surface`
- `--color-surface-2`
- `--color-border`
- `--color-text`
- `--color-text-muted`
- `--color-primary`
- `--color-primary-hover`

这些 token 是主题的最小语义层。主壳通过桥接把它们映射成工作台通用变量。

## Dockview Token

如果扩展要完整接管工作台分栏观感，应同时提供以下 token：

- `--dockview-header-surface`
- `--dockview-header-border`
- `--dockview-tab-surface`
- `--dockview-tab-hover`
- `--dockview-tab-border`
- `--dockview-tab-accent`
- `--dockview-drop-surface`
- `--dockview-tab-shadow`
- `--dockview-divider-shadow`
- `--dockview-splitter-line`
- `--dockview-splitter-track`
- `--dockview-splitter-hover`
- `--dockview-empty-surface`
- `--dockview-empty-border`
- `--dockview-empty-accent`
- `--dockview-empty-card`
- `--dockview-empty-card-hover`

未声明这些 token 时，主壳会继续使用基础 token 派生的默认映射。

## 主壳语义变量

主壳页面与布局直接依赖的是这些语义变量：

- `--bg`
- `--bg-elevated`
- `--bg-soft`
- `--bg-panel`
- `--line`
- `--line-strong`
- `--text`
- `--text-muted`
- `--accent`
- `--accent-soft`

扩展主题不应直接把这些变量当成自己的主入口；推荐始终先定义 `--color-*`，再让 runtime bridge 完成映射。

## Manifest 示例

```toml
[capabilities]
themes = true

[contributes]
themes = [
  {
    id = "acme.sunrise",
    contract = "ennoia.theme",
    label = { key = "ext.acme.theme.sunrise", fallback = "Sunrise" },
    appearance = "light",
    tokens_entry = "ui/themes/sunrise.css",
    preview_color = "#f59e0b",
    extends = "system",
    category = "extension"
  }
]
```

## CSS 示例

```css
:root[data-theme="acme.sunrise"] {
  --color-bg: #fff7ed;
  --color-surface: #fffbf5;
  --color-surface-2: #fde7cf;
  --color-border: #efc38d;
  --color-text: #3b2414;
  --color-text-muted: #8b6a52;
  --color-primary: #ea580c;
  --color-primary-hover: #f97316;

  --dockview-header-surface: rgba(255, 251, 245, 0.88);
  --dockview-header-border: #efc38d;
  --dockview-tab-surface: rgba(255, 255, 255, 0.92);
  --dockview-tab-hover: rgba(253, 231, 207, 0.42);
  --dockview-tab-border: #efc38d;
  --dockview-tab-accent: #ea580c;
}
```

## 约束

- 主题 CSS 只定义变量，不直接覆盖主壳页面结构 class
- 不依赖具体 DOM 层级、组件私有 class 或第三方库临时 class 名
- `extends` 只表达视觉基底继承，不表达业务语义
- 如果主题需要额外效果，优先通过新增稳定 token 扩展协议，而不是直接写选择器补丁

## 运行时行为

- 内建主题与扩展主题都统一进入 `@ennoia/theme-runtime`
- runtime 会校验 contract 是否受支持
- 扩展主题必须通过 `cssUrl` stylesheet 方式注入
- 主壳最终应用的是“基础主题变量 + 扩展 stylesheet 覆盖”的组合结果
