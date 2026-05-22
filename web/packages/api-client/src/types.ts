import type {
  ExtensionBehaviorContribution,
  ExtensionDiagnostic,
  ExtensionEvent,
  ExtensionEventContribution,
  ExtensionActionContribution,
  ExtensionHookContribution,
  ExtensionLocaleContribution,
  ExtensionMemoryContribution,
  ExtensionOperation,
  ExtensionOperationContribution,
  ExtensionPageContribution,
  ExtensionPanelContribution,
  ExtensionProviderContribution,
  ExtensionScheduleActionContribution,
  ExtensionSettingField,
  ExtensionThemeContribution,
  ExtensionView,
  ExtensionViewContribution,
  LocalizedText,
} from "@ennoia/ui-sdk";

export type BootstrapState = {
  is_initialized: boolean;
  initialized_at?: string | null;
};

export type UiPreference = {
  locale?: string | null;
  theme_id?: string | null;
  time_zone?: string | null;
  date_style?: string | null;
  density?: string | null;
  motion?: string | null;
  version: number;
  updated_at: string;
};

export type UiPreferenceRecord = {
  subject_id: string;
  preference: UiPreference;
};

export type UiConfig = {
  web_title: LocalizedText;
  default_theme: string;
  default_locale: string;
  fallback_locale: string;
  available_locales: string[];
  default_display_name: string;
  default_time_zone: string;
  show_command_palette: boolean;
  api: {
    default_request_timeout_ms?: number | null;
  };
  notifications: {
    success_auto_dismiss_ms: number;
    error_auto_dismiss_ms: number;
    pause_on_hover: boolean;
  };
};

export type UiRuntime = {
  ui_config: UiConfig;
  registry: {
    views: ExtensionViewContribution[];
    operations: ExtensionOperationContribution[];
    events: ExtensionEventContribution[];
    pages: ExtensionPageContribution[];
    panels: ExtensionPanelContribution[];
    themes: ExtensionThemeContribution[];
    locales: ExtensionLocaleContribution[];
    providers: ExtensionProviderContribution[];
    behaviors: ExtensionBehaviorContribution[];
    memories: ExtensionMemoryContribution[];
    hooks: ExtensionHookContribution[];
    actions: ExtensionActionContribution[];
    schedule_actions: ExtensionScheduleActionContribution[];
  };
  instance_preference?: UiPreferenceRecord | null;
  space_preferences: UiPreferenceRecord[];
  versions: {
    registry: number;
    preferences: number;
  };
};

export type UiMessageBundle = {
  locale: string;
  resolved_locale: string;
  namespace: string;
  messages: Record<string, string>;
  source: string;
  version: string;
};

export type UiMessagesResponse = {
  locale: string;
  fallback_locale: string;
  bundles: UiMessageBundle[];
};

export type RuntimeProfile = {
  id: string;
  display_name: string;
  locale: string;
  time_zone: string;
  operating_system?: string | null;
  default_space_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ServerConfig = {
  host: string;
  port: number;
  web_dev: {
    host: string;
    port: number;
  };
  rate_limit: {
    enabled: boolean;
    per_ip_rpm: number;
    per_user_rpm: number;
    burst: number;
    exempt_paths: string[];
  };
  cors: {
    enabled: boolean;
    origins: string[];
    methods: string[];
    credentials: boolean;
    max_age_seconds: number;
  };
  timeout: {
    enabled: boolean;
    default_ms: number;
    per_path_ms: Record<string, number>;
  };
  operations: {
    command: {
      default_timeout_ms: number;
      min_timeout_ms: number;
      max_timeout_ms: number;
    };
    net: {
      default_timeout_ms: number;
      min_timeout_ms: number;
      max_timeout_ms: number;
    };
  };
  providers: {
    default_request_timeout_ms: number;
  };
  streams: {
    conversation_poll_ms: number;
    workflow_poll_ms: number;
    logs_poll_ms: number;
  };
  background: {
    extension_refresh_ms: number;
    schedule_tick_ms: number;
    event_delivery_tick_ms: number;
  };
  extension_runtime: {
    timeout_ms: number;
    memory_limit_mb: number;
  };
  schedules: {
    command: {
      default_timeout_ms: number;
      min_timeout_ms: number;
      max_timeout_ms: number;
    };
    retry: {
      default_max_attempts: number;
      max_attempts_cap: number;
      default_backoff_seconds: number;
      max_backoff_seconds: number;
    };
  };
  dev_supervisor: {
    host_reload_debounce_ms: number;
    watch_poll_ms: number;
    api_ready_timeout_ms: number;
    api_healthcheck_interval_ms: number;
    api_healthcheck_grace_ms: number;
    api_port_release_timeout_ms: number;
    child_startup_grace_ms: number;
    probe_socket_timeout_ms: number;
  };
  logging: {
    enabled: boolean;
    level: string;
    sample_rate: number;
    redact_headers: string[];
    dev_console: {
      enabled: boolean;
      level: string;
    };
  };
  body_limit: {
    enabled: boolean;
    max_bytes: number;
    per_path_max: Record<string, number>;
  };
  bootstrap: BootstrapState;
};

export type BootstrapSetupResponse = {
  bootstrap: BootstrapState;
  profile: RuntimeProfile;
  preference: UiPreferenceRecord;
};

export type AgentProfile = {
  id: string;
  display_name: string;
  description: string;
  system_prompt: string;
  model_endpoint_id: string;
  model_id: string;
  generation_options: Record<string, string>;
  skills: string[];
  enabled: boolean;
  kind?: string;
  default_model?: string;
  skills_dir?: string;
  working_dir?: string;
  artifacts_dir?: string;
  permission_profile: AgentPermissionProfile;
  file_access: AgentFileAccessProfile;
};

export type AgentFileAccessProfile = {
  default_root: string;
  roots: AgentFileAccessRoot[];
};

export type AgentFileAccessRoot = {
  id: string;
  path: string;
  mode: string;
};

export type SkillConfig = {
  id: string;
  version: string;
  description: string;
  mount: {
    mode: string;
  };
  actions: SkillActionConfig[];
  enabled: boolean;
  builtin_sync_blocked: boolean;
  settings: ExtensionSettingField[];
  diagnostics: SkillDiagnosticsSpec;
  prepare?: SkillCommandSpec | null;
  readiness: SkillReadinessSummary;
};

export type SkillActionConfig = {
  id: string;
  description: string;
  entry: string;
};

export type SkillDiagnosticsSpec = {
  manual_check: boolean;
  check?: SkillCommandSpec | null;
};

export type SkillCommandSpec = {
  runner: string;
  entry: string;
  args: string[];
  timeout_ms?: number | null;
};

export type SkillReadinessSummary = {
  status: SkillRuntimeStatus;
  summary: string;
  checked_at?: string | null;
};

export type SkillRuntimeStatus =
  | "ready"
  | "partial"
  | "missing_config"
  | "env_missing"
  | "error"
  | "unknown";

export type SkillCheckCategory =
  | "config"
  | "environment"
  | "permission"
  | "dependency"
  | "connectivity";

export type SkillCheckItemStatus =
  | "ok"
  | "missing"
  | "warning"
  | "error"
  | "skipped";

export type SkillCheckItem = {
  key: string;
  category: SkillCheckCategory;
  label: string;
  status: SkillCheckItemStatus;
  required: boolean;
  message?: string | null;
  fix_hint?: string | null;
};

export type SkillCheckAction = {
  key: string;
  label: string;
  kind: string;
};

export type SkillCheckResult = {
  status: SkillRuntimeStatus;
  summary: string;
  checked_at?: string | null;
  items: SkillCheckItem[];
  actions: SkillCheckAction[];
};

export type SkillSettingsResponse = {
  skill_id: string;
  values: Record<string, string | number | boolean>;
};

export type ProviderModelDescriptor = {
  id: string;
  max_context_tokens?: number | null;
  max_input_tokens?: number | null;
};

export type ModelEndpointConfig = {
  id: string;
  display_name: string;
  kind: string;
  description: string;
  base_url: string;
  api_key: string;
  api_key_env: string;
  request_timeout_ms?: number | null;
  default_model: string;
  available_models: ProviderModelDescriptor[];
  model_discovery: {
    manual_allowed: boolean;
  };
  enabled: boolean;
};

export type ModelEndpointModelsResponse = {
  model_endpoint_id: string;
  source: string;
  models: ProviderModelDescriptor[];
  manual_allowed: boolean;
  generation_options: ExtensionProviderContribution["provider"]["generation_options"];
};


export type PermissionTarget = {
  kind: string;
  id: string;
  conversation_id?: string | null;
  run_id?: string | null;
  path?: string | null;
  host?: string | null;
};

export type PermissionScope = {
  conversation_id?: string | null;
  run_id?: string | null;
  message_id?: string | null;
  extension_id?: string | null;
  path?: string | null;
  host?: string | null;
};

export type PermissionTrigger = {
  kind: string;
  user_initiated: boolean;
};

export type AgentPermissionProfile = {
  mode: string;
  entries: AgentPermissionCommandEntry[];
};

export type AgentPermissionCommandEntry = {
  match: string;
  value: string;
};

export type PermissionPolicySummary = {
  agent_id: string;
  mode: string;
  allow_count: number;
  ask_count: number;
  deny_count: number;
};

export type PermissionEventRecord = {
  event_id: string;
  agent_id: string;
  action: string;
  decision: string;
  target: PermissionTarget;
  scope: PermissionScope;
  extension_id?: string | null;
  matched_rule_id?: string | null;
  approval_id?: string | null;
  trace_id?: string | null;
  created_at: string;
};

export type PermissionApprovalRecord = {
  approval_id: string;
  status: string;
  agent_id: string;
  action: string;
  target: PermissionTarget;
  scope: PermissionScope;
  trigger: PermissionTrigger;
  matched_rule_id?: string | null;
  reason: string;
  created_at: string;
  expires_at?: string | null;
  resolved_at?: string | null;
  resolution?: string | null;
};

export type PermissionRequest = {
  agent_id: string;
  action: string;
  target: PermissionTarget;
  scope: PermissionScope;
  trigger: PermissionTrigger;
};

export type PermissionGrantRecord = {
  grant_id: string;
  approval_id: string;
  agent_id: string;
  mode: string;
  request: PermissionRequest;
  consumed_at?: string | null;
  expires_at?: string | null;
  revoked_at?: string | null;
};

export type OperationStatus =
  | "queued"
  | "running"
  | "blocked"
  | "succeeded"
  | "failed"
  | "cancelled";

export type OperationRecord = {
  id: string;
  extension_id: string;
  agent_id: string;
  conversation_id: string;
  branch_id?: string | null;
  lane_id?: string | null;
  run_id: string;
  message_id?: string | null;
  kind: string;
  name: string;
  status: OperationStatus;
  input: unknown;
  output?: unknown;
  error?: unknown;
  created_at: string;
  updated_at: string;
};

export type ConversationSummary = {
  id: string;
  topology: "direct" | "group";
  owner: { kind: string; id: string };
  space_id?: string | null;
  title: string;
  participants: string[];
  active_branch_id?: string | null;
  default_lane_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ConversationBranch = {
  id: string;
  conversation_id: string;
  name: string;
  kind: string;
  status: string;
  parent_branch_id?: string | null;
  source_message_id?: string | null;
  inherit_mode: string;
  created_at: string;
  updated_at: string;
  is_active?: boolean;
  depth?: number;
  own_message_count?: number;
  visible_message_count?: number;
  last_message_at?: string | null;
  last_activity_at?: string | null;
  source_preview?: string | null;
};

export type ConversationLane = {
  id: string;
  conversation_id: string;
  space_id?: string | null;
  name: string;
  lane_type: string;
  status: string;
  goal: string;
  participants: string[];
  created_at: string;
  updated_at: string;
};

export type ConversationMessage = {
  id: string;
  conversation_id: string;
  branch_id?: string | null;
  lane_id?: string | null;
  sender: string;
  role: "operator" | "agent" | "system" | "tool";
  body: string;
  mentions: string[];
  parent_message_id?: string | null;
  reply_to_message_id?: string | null;
  rewrite_from_message_id?: string | null;
  created_at: string;
};

export type ExecutionRun = {
  id: string;
  owner: { kind: string; id: string };
  conversation_id: string;
  lane_id?: string | null;
  source_message_id?: string | null;
  trigger: string;
  stage: string;
  goal: string;
  created_at: string;
  updated_at: string;
};

export type ExecutionStep = {
  id: string;
  run_id: string;
  conversation_id: string;
  lane_id?: string | null;
  task_kind: string;
  title: string;
  assigned_agent_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type RunOutput = {
  id: string;
  owner: { kind: string; id: string };
  run_id: string;
  conversation_id?: string | null;
  lane_id?: string | null;
  kind: string;
  relative_path: string;
  created_at: string;
};

export type ConversationDetail = {
  conversation: ConversationSummary;
  lanes: ConversationLane[];
  branches: ConversationBranch[];
  messages: ConversationMessage[];
  records: ExtensionRecordEntry[];
  operations: OperationRecord[];
  runs: ExecutionRun[];
  tasks: ExecutionStep[];
  outputs: RunOutput[];
};

export type ConversationStreamSnapshot = {
  detail: ConversationDetail;
  approvals: PermissionApprovalRecord[];
  operations: OperationRecord[];
};

export type ConversationMessageAppendResponse = {
  conversation: ConversationSummary;
  lane: ConversationLane;
  branch: ConversationBranch;
  message: ConversationMessage;
  run?: ExecutionRun;
  runs?: ExecutionRun[];
  tasks: ExecutionStep[];
  artifacts: RunOutput[];
};


export type ExtensionRuntimeState = {
  id: string;
  name: string;
  enabled: boolean;
  status: string;
  source_mode: string;
  install_dir: string;
  source_root: string;
  diagnostics: ExtensionDiagnostic[];
};

export type ExtensionRuntimeEvent = {
  event_id: string;
  extension_id?: string | null;
  generation: number;
  event: string;
  health?: string | null;
  summary: string;
  diagnostics: ExtensionDiagnostic[];
  occurred_at: string;
};

export type ExtensionDetail = {
  id: string;
  version?: string | null;
  name: string;
  description: string;
  docs?: string | null;
  compat: {
    ennoia?: string | null;
  };
  conversation: {
    visible: boolean;
    resources: string[];
    operations: string[];
  };
  source_mode: string;
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  views: ExtensionView[];
  operations: ExtensionOperation[];
  events: ExtensionEvent[];
  settings: ExtensionSettingField[];
  diagnostics: ExtensionDiagnostic[];
};

export type ExtensionSettingsResponse = {
  extension_id: string;
  values: Record<string, string | number | boolean>;
};

export type ExtensionStateEntry = {
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  key: string;
  value: unknown;
  version: number;
  updated_at: string;
  expires_at?: string | null;
};

export type ExtensionRecordEntry = {
  id: string;
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  kind: string;
  status?: string | null;
  title?: string | null;
  summary?: string | null;
  payload: unknown;
  related_message_id?: string | null;
  parent_id?: string | null;
  created_at: string;
  updated_at: string;
  closed_at?: string | null;
};

export type SystemLog = {
  id: string;
  kind?: string;
  source: string;
  level: string;
  title: string;
  summary: string;
  details?: string | null;
  run_id?: string | null;
  task_id?: string | null;
  at: string;
};

export type LogsOverview = {
  log_count: number;
  span_count: number;
  trace_count: number;
};

export type LogEntry = {
  id: string;
  seq: number;
  event: string;
  level: string;
  component: string;
  source_kind: string;
  source_id?: string | null;
  request_id?: string | null;
  trace_id?: string | null;
  span_id?: string | null;
  parent_span_id?: string | null;
  message: string;
  attributes: unknown;
  created_at: string;
};

export type LogTraceRecord = {
  id: string;
  seq: number;
  trace_id: string;
  span_id: string;
  parent_span_id?: string | null;
  request_id: string;
  sampled: boolean;
  source: string;
  kind: string;
  name: string;
  component: string;
  source_kind: string;
  source_id?: string | null;
  status: string;
  attributes: unknown;
  started_at: string;
  ended_at: string;
  duration_ms: number;
};

export type LogTraceLinkRecord = {
  id: string;
  seq: number;
  trace_id: string;
  span_id: string;
  linked_trace_id: string;
  linked_span_id: string;
  link_type: string;
  attributes: unknown;
  created_at: string;
};

export type LogTraceDetail = {
  trace_id: string;
  spans: LogTraceRecord[];
  links: LogTraceLinkRecord[];
};

export type LogStreamDelta = {
  overview: LogsOverview;
  logs: LogEntry[];
  traces: LogTraceRecord[];
};

export type LogEntryQuery = {
  event?: string;
  level?: string;
  component?: string;
  source_kind?: string;
  source_id?: string;
  request_id?: string;
  trace_id?: string;
  cursor?: number;
  limit?: number;
};

export type LogTraceQuery = {
  request_id?: string;
  component?: string;
  kind?: string;
  source_kind?: string;
  source_id?: string;
  limit?: number;
};

export type ActionImplementation = {
  extension_id: string;
  operation: string;
  method: string;
  phase: string;
  priority: number;
  enabled: boolean;
  result_mode: string;
  when: unknown;
  schema?: string | null;
  extension_status: string;
};

export type ActionStatus = {
  action: string;
  rules: ActionImplementation[];
  execute_rule_count: number;
};

export type ScheduleTrigger =
  | { kind: "once"; at: string }
  | { kind: "interval"; every_seconds: number }
  | { kind: "cron"; expression: string; next_run_at: string };

export type ScheduleExecutor =
  | {
      kind: "command";
      command: {
        command: string;
        cwd?: string | null;
        timeout_ms?: number | null;
      };
    }
  | {
      kind: "agent";
      agent: {
        agent_id: string;
        prompt: string;
        model_id?: string | null;
        max_turns?: number | null;
        context?: {
          conversation_id?: string | null;
        };
      };
    };

export type ScheduleDelivery = {
  conversation_id?: string | null;
  lane_id?: string | null;
  content_mode?: "full" | "summary" | "conclusion" | null;
};

export type ScheduleRetryPolicy = {
  max_attempts?: number;
  backoff_seconds?: number;
};

export type ScheduleRunRecord = {
  id: string;
  started_at: string;
  finished_at: string;
  attempt: number;
  status: string;
  error?: string | null;
  delivered?: boolean;
  delivery_error?: string | null;
  output?: unknown;
};

export type ScheduleRecord = {
  id: string;
  name?: string | null;
  description?: string | null;
  owner: unknown;
  trigger: ScheduleTrigger;
  executor: ScheduleExecutor;
  delivery?: ScheduleDelivery;
  retry?: ScheduleRetryPolicy;
  enabled: boolean;
  next_run_at?: string | null;
  last_run_at?: string | null;
  last_status?: string | null;
  last_error?: string | null;
  last_output?: unknown;
  history?: ScheduleRunRecord[];
  created_at: string;
  updated_at: string;
};

export type SchedulePayload = {
  name?: string | null;
  description?: string | null;
  owner?: unknown;
  trigger: ScheduleTrigger;
  executor: ScheduleExecutor;
  delivery?: ScheduleDelivery;
  retry?: ScheduleRetryPolicy;
  enabled?: boolean;
};

export type SystemLogEntry = {
  id: string;
  seq: number;
  event: string;
  level: string;
  component: string;
  source_kind: string;
  source_id?: string | null;
  summary: string;
  payload: unknown;
  created_at: string;
};



