# Ennoia 配置模型

## 1. 顶层配置文件

### `config/ennoia.toml`

全局配置字段建议：

- `app_name`
- `mode`
- `database_mode`：首版默认值为 `sqlite`
- `database_url`：默认指向 `sqlite://~/.ennoia/state/sqlite/ennoia.db`
- `extensions_scan_dir`
- `agents_scan_dir`
- `scheduler_tick_ms`
- `default_mention_mode`

### `config/server.toml`

- `host`
- `port`
- `log_level`
- `allow_origins`
- `enable_ws`

### `config/ui.toml`

- `shell_title`：`LocalizedText` 结构，包含 `key` 与 `fallback`
- `default_theme`
- `default_locale`
- `fallback_locale`
- `available_locales`
- `dock_persistence`
- `default_page`
- `show_command_palette`

## 2. Agent 配置

每个 Agent 一个文件：

```toml
id = "coder"
display_name = "Coder"
kind = "agent"
workspace_mode = "private"
default_model = "gpt-5.4"
skills_dir = "~/.ennoia/agents/coder/skills"
workspace_dir = "~/.ennoia/agents/coder/workspace"
artifacts_dir = "~/.ennoia/agents/coder/artifacts"
```

## 3. Extension 配置

每个 system extension 一个文件：

```toml
id = "observatory"
kind = "system"
enabled = true
install_dir = "~/.ennoia/global/extensions/observatory"
```

## 4. Space 策略

Space 配置可在数据库或后续专门的配置目录中保存，首版模型至少需要：

- `mention_mode`
- `default_agents`
- `allow_auto_reply`

## 5. 配置加载原则

- 所有路径支持目录扫描
- 配置文件彼此职责明确
- Agent 与 Extension 通过独立文件管理
- 配置是数据库外的启动事实源
