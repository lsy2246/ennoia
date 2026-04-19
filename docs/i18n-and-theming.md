# Ennoia 多语言与多主题

## 原则

- 多语言与多主题是实例级正式能力
- 首屏优先读取浏览器本地缓存
- 服务端保存实例级与空间级偏好
- 扩展通过 registry 贡献 `theme` 与 `locale`
- 前端消息按 feature / namespace 分模块组织

## 持久化

SQLite 中的正式偏好表：

- `instance_ui_preferences`
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

## API

- `GET /api/v1/ui/runtime`
- `GET /api/v1/ui/messages`
- `GET /api/v1/runtime/preferences`
- `PUT /api/v1/runtime/preferences`
- `GET /api/v1/spaces/{space_id}/ui-preferences`
- `PUT /api/v1/spaces/{space_id}/ui-preferences`

`/api/v1/ui/runtime` 返回：

- `ui_config`
- `registry.pages`
- `registry.panels`
- `registry.themes`
- `registry.locales`
- `instance_preference`
- `space_preferences`
- `versions`

`/api/v1/ui/messages` 返回：

- `locale`
- `fallback_locale`
- `bundles[]`

## 前端策略

1. 启动前读取本地缓存 `ennoia.ui.bootstrap`
2. 在 React 挂载前先应用主题和 `lang`
3. 挂载后再请求 `/api/v1/ui/runtime`
4. 用户修改偏好时先写浏览器缓存，再同步服务端

## 消息组织

`@ennoia/i18n` 采用“模块分文件 + 单入口注册 + runtime registry”的组织方式：

- `shell`
- `settings`
- `ext.observatory`

每个模块独立维护自己的 namespace 与文案表，统一由 registry 注册。
Shell 启动后会按当前 locale 请求 `/api/v1/ui/messages`，把服务端返回的 bundle 注册到前端 runtime registry。
这样页面、内建扩展和后续插件都可以共用同一套消息协议。
