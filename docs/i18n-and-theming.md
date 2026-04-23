# Ennoia 多语言与多主题

## 原则

- 多语言与多主题是实例级正式能力
- 首屏优先读取浏览器本地缓存
- 服务端把实例级与空间级偏好保存为文件
- 扩展通过 registry 贡献 `theme` 与 `locale`
- 前端消息按 feature / namespace 分模块组织

## 持久化

运行目录中的正式偏好文件：

- `config/preferences/instance.toml`
- `config/preferences/spaces/<space_id>.toml`

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

- `GET /api/ui/runtime`
- `GET /api/ui/messages`
- `GET /api/runtime/preferences`
- `PUT /api/runtime/preferences`
- `GET /api/spaces/{space_id}/ui-preferences`
- `PUT /api/spaces/{space_id}/ui-preferences`

`/api/ui/runtime` 返回：

- `ui_config`
- `registry.pages`
- `registry.panels`
- `registry.themes`
- `registry.locales`
- `instance_preference`
- `space_preferences`
- `versions`

`/api/ui/messages` 返回：

- `locale`
- `fallback_locale`
- `bundles[]`

## 前端策略

1. 启动前读取本地缓存 `ennoia.ui.bootstrap`
2. 在 React 挂载前先应用主题和 `lang`
3. 挂载后再请求 `/api/ui/runtime`
4. 主题切换先立即写浏览器缓存并应用，再同步服务端
5. 语言只允许从 `ui_config.available_locales` 中选择，首次引导页与设置页保持一致
6. 时区只作为时间展示偏好；首次引导默认浏览器时区，设置页支持清空为浏览器默认

## 消息组织

`@ennoia/i18n` 采用“模块分文件 + 单入口注册 + runtime registry”的组织方式：

- `web`
- `settings`
- `ext.observatory`

每个模块独立维护自己的 namespace 与文案表，统一由 registry 注册。
Web 启动后会按当前 locale 请求 `/api/ui/messages`，把服务端返回的 bundle 注册到前端 runtime registry。
这样页面、内建扩展和后续插件都可以共用同一套消息协议。

## 当前交互约定

- 首次引导页不再允许自由输入 locale，而是复用实例支持语言下拉框
- 首次引导页允许选择完整 IANA 时区列表，默认使用浏览器检测结果
- 设置页主题选择即刻生效，不依赖重新打开页面或重启服务
- 主题运行时同时支持内建主题与扩展贡献主题；扩展主题样式通过 API 以 stylesheet 方式加载
