import type {
  ExtensionBehaviorContribution,
  ExtensionDiagnostic,
  ExtensionInterfaceContribution,
  ExtensionLocaleContribution,
  ExtensionMemoryContribution,
  ExtensionPageContribution,
  ExtensionPanelContribution,
  ExtensionProviderContribution,
  ExtensionScheduleActionContribution,
  ExtensionThemeContribution,
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
  dock_persistence: boolean;
  default_page: string;
  show_command_palette: boolean;
};

export type UiRuntime = {
  ui_config: UiConfig;
  registry: {
    pages: ExtensionPageContribution[];
    panels: ExtensionPanelContribution[];
    themes: ExtensionThemeContribution[];
    locales: ExtensionLocaleContribution[];
    providers: ExtensionProviderContribution[];
    behaviors: ExtensionBehaviorContribution[];
    memories: ExtensionMemoryContribution[];
    interfaces: ExtensionInterfaceContribution[];
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
  default_space_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type AppConfig = {
  app_name: string;
  mode: string;
  default_mention_mode: string;
};

export type ServerConfig = {
  host: string;
  port: number;
  enable_ws: boolean;
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
  logging: {
    enabled: boolean;
    level: string;
    sample_rate: number;
    redact_headers: string[];
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
  provider_id: string;
  model_id: string;
  generation_options: Record<string, string>;
  skills: string[];
  enabled: boolean;
  kind?: string;
  default_model?: string;
  skills_dir?: string;
  working_dir?: string;
  artifacts_dir?: string;
};

export type SkillConfig = {
  id: string;
  display_name: string;
  description: string;
  source: string;
  entry: string;
  tags: string[];
  enabled: boolean;
};

export type ProviderConfig = {
  id: string;
  display_name: string;
  kind: string;
  description: string;
  base_url: string;
  api_key_env: string;
  default_model: string;
  available_models: string[];
  model_discovery: {
    mode: string;
    manual_allowed: boolean;
  };
  enabled: boolean;
};

export type ProviderModelsResponse = {
  provider_id: string;
  source: string;
  models: string[];
  recommended_model?: string | null;
  manual_allowed: boolean;
  generation_options: ExtensionProviderContribution["provider"]["generation_options"];
};

export type ChatThread = {
  id: string;
  topology: "direct" | "group";
  owner: { kind: string; id: string };
  space_id?: string | null;
  title: string;
  participants: string[];
  default_lane_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ChatLane = {
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

export type ChatMessage = {
  id: string;
  conversation_id: string;
  lane_id?: string | null;
  sender: string;
  role: "operator" | "agent" | "system" | "tool";
  body: string;
  mentions: string[];
  created_at: string;
};

export type ExecutionRun = {
  id: string;
  owner: { kind: string; id: string };
  conversation_id: string;
  lane_id?: string | null;
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

export type ChatThreadDetail = {
  conversation: ChatThread;
  lanes: ChatLane[];
  messages: ChatMessage[];
  runs: ExecutionRun[];
  tasks: ExecutionStep[];
  outputs: RunOutput[];
};

export type ChatSendResponse = {
  conversation: ChatThread;
  lane: ChatLane;
  message: ChatMessage;
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
  version: string;
  kind: string;
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
  name: string;
  kind: string;
  version: string;
  source_mode: string;
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  diagnostics: ExtensionDiagnostic[];
  ui?: {
    kind: string;
    entry: string;
    hmr: boolean;
    version: string;
  } | null;
  worker?: {
    kind: string;
    entry: string;
    abi: string;
    status: string;
  } | null;
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

export type InterfaceImplementation = {
  extension_id: string;
  method: string;
  version: string;
  schema?: string | null;
  extension_status: string;
};

export type InterfaceStatus = {
  key: string;
  implementations: InterfaceImplementation[];
  active?: InterfaceImplementation | null;
  status: string;
};

export type InterfaceBinding = {
  extension_id: string;
  method: string;
};

export type InterfaceBindings = {
  bindings: Record<string, InterfaceBinding>;
};

export type ScheduleTrigger =
  | { kind: "once"; at: string }
  | { kind: "interval"; every_seconds: number }
  | { kind: "cron"; expression: string; next_run_at: string };

export type ExtensionScheduleTarget = {
  kind?: "extension";
  extension_id: string;
  action_id: string;
};

export type CommandScheduleTarget = {
  kind: "command";
  command: {
    command: string;
    cwd?: string | null;
    timeout_ms?: number | null;
  };
};

export type ScheduleTarget = ExtensionScheduleTarget | CommandScheduleTarget;

export type ScheduleRecord = {
  id: string;
  owner: unknown;
  trigger: ScheduleTrigger;
  target: ScheduleTarget;
  params: unknown;
  enabled: boolean;
  next_run_at?: string | null;
  last_run_at?: string | null;
  last_status?: string | null;
  last_error?: string | null;
  created_at: string;
  updated_at: string;
};

export type SchedulePayload = {
  owner?: unknown;
  trigger: ScheduleTrigger;
  target: ScheduleTarget;
  params?: unknown;
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



