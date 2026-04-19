# Ennoia 多语言与多主题

## 1. 总体原则

- 多语言与多主题是平台正式能力，不是页面级补丁
- 浏览器本地缓存负责首屏启动
- 服务端偏好负责跨设备同步
- `shell` 负责运行时应用
- `extension-host` 负责注册 theme / locale contribution

## 2. 协议层

当前 `kernel` 已引入以下正式模型：

- `LocalizedText`：所有 UI 标题、标签的稳定本地化协议
- `ThemeAppearance`：主题视觉类别
- `LocaleContribution`：扩展贡献的语言包描述
- `UiPreference`：用户或 Space 的 UI 偏好

扩展 manifest 中以下字段已经改为本地化协议：

- `pages[].title`
- `panels[].title`
- `commands[].title`
- `themes[].label`

## 3. 配置层

`config/ui.toml` 现在定义实例级默认 UI 行为：

- `shell_title`
- `default_theme`
- `default_locale`
- `fallback_locale`
- `available_locales`
- `dock_persistence`
- `default_page`
- `show_command_palette`

实例默认值只作为 fallback，不替代用户或 Space 偏好。

## 4. 持久化层

SQLite 现在正式包含：

- `user_ui_preferences`
- `space_ui_preferences`

字段覆盖：

- `locale`
- `theme_id`
- `time_zone`
- `date_style`
- `density`
- `motion`
- `version`
- `updated_at`

## 5. API

当前新增正式接口：

- `GET /api/v1/ui/runtime`
- `GET /api/v1/me/ui-preferences`
- `PUT /api/v1/me/ui-preferences`
- `GET /api/v1/spaces/{space_id}/ui-preferences`
- `PUT /api/v1/spaces/{space_id}/ui-preferences`

其中 `ui/runtime` 返回：

- `ui_config`
- `registry.pages`
- `registry.panels`
- `registry.themes`
- `registry.locales`
- `user_preference`
- `space_preferences`
- `versions`

## 6. 前端运行时

前端新增两个正式 runtime 包：

- `@ennoia/i18n`
- `@ennoia/theme-runtime`

当前策略：

1. 启动时优先读取浏览器本地缓存 `ennoia.ui.bootstrap`
2. 在 React 挂载前应用 `theme` 与 `lang`
3. App 启动后请求 `/api/v1/ui/runtime`
4. 用服务端快照补齐 runtime registry 与远端偏好
5. 用户在设置页修改偏好后，立即写本地缓存并异步同步服务端

## 7. 优先级

语言优先级：

1. 浏览器本地缓存
2. 当前用户偏好
3. Space 偏好
4. `ui.toml.default_locale`
5. `ui.toml.fallback_locale`

主题优先级：

1. 浏览器本地缓存
2. 当前用户偏好
3. Space 偏好
4. `ui.toml.default_theme`
5. 内建 `system`
