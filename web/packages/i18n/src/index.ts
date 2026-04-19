import type { LocalizedText } from "@ennoia/ui-sdk";

export type TranslationBundle = Record<string, string>;

const CORE_MESSAGES: Record<string, TranslationBundle> = {
  "zh-CN": {
    "shell.title": "Ennoia",
    "nav.dashboard": "概览",
    "nav.memories": "记忆",
    "nav.settings": "设置",
    "nav.users": "用户",
    "nav.sessions": "会话",
    "nav.api_keys": "API Keys",
    "auth.logout": "退出登录",
    "auth.login.title": "登录 Ennoia",
    "auth.login.subtitle": "登录后继续使用工作台",
    "auth.login.username": "用户名",
    "auth.login.password": "密码",
    "auth.login.submit": "登录",
    "auth.login.submitting": "登录中…",
    "dashboard.title": "概览",
    "dashboard.loading": "正在加载…",
    "dashboard.recent_runs": "最近运行",
    "dashboard.agents": "Agents",
    "dashboard.details": "详情",
    "settings.title": "设置",
    "settings.personal.title": "个人界面偏好",
    "settings.personal.subtitle": "这里的选择会保存在浏览器本地，并在后台同步到当前账号。",
    "settings.personal.theme": "主题",
    "settings.personal.locale": "语言",
    "settings.personal.time_zone": "时区",
    "settings.personal.date_style": "日期格式",
    "settings.personal.save": "保存个人偏好",
    "settings.personal.saved": "个人偏好已保存。",
    "settings.system.title": "系统运行时配置",
    "settings.system.subtitle": "管理员可以直接编辑中间件与鉴权配置。",
    "settings.reload": "重新加载",
    "settings.save_apply": "保存并立即应用",
    "settings.recent_changes": "最近变更",
    "common.loading": "加载中…",
    "common.saved": "保存成功。",
    "common.unknown": "未知",
    "date_style.locale": "跟随语言",
    "date_style.iso": "ISO 8601",
    "theme.system": "跟随系统",
    "theme.midnight": "Midnight",
    "theme.paper": "Paper",
    "theme.daybreak": "Daybreak",
    "ext.observatory.page.events": "观测台",
    "ext.observatory.panel.timeline": "事件时间线",
    "ext.observatory.theme.daybreak": "Daybreak",
    "ext.observatory.command.open": "打开观测台"
  },
  "en-US": {
    "shell.title": "Ennoia",
    "nav.dashboard": "Dashboard",
    "nav.memories": "Memories",
    "nav.settings": "Settings",
    "nav.users": "Users",
    "nav.sessions": "Sessions",
    "nav.api_keys": "API Keys",
    "auth.logout": "Logout",
    "auth.login.title": "Sign in to Ennoia",
    "auth.login.subtitle": "Sign in to continue to the workspace",
    "auth.login.username": "Username",
    "auth.login.password": "Password",
    "auth.login.submit": "Sign in",
    "auth.login.submitting": "Signing in…",
    "dashboard.title": "Dashboard",
    "dashboard.loading": "Loading…",
    "dashboard.recent_runs": "Recent runs",
    "dashboard.agents": "Agents",
    "dashboard.details": "Details",
    "settings.title": "Settings",
    "settings.personal.title": "Personal UI preferences",
    "settings.personal.subtitle": "These choices are cached in the browser and synchronized to the current account in the background.",
    "settings.personal.theme": "Theme",
    "settings.personal.locale": "Language",
    "settings.personal.time_zone": "Time zone",
    "settings.personal.date_style": "Date format",
    "settings.personal.save": "Save preferences",
    "settings.personal.saved": "Preferences saved.",
    "settings.system.title": "System runtime config",
    "settings.system.subtitle": "Administrators can edit middleware and auth configuration live.",
    "settings.reload": "Reload",
    "settings.save_apply": "Save & apply",
    "settings.recent_changes": "Recent changes",
    "common.loading": "Loading…",
    "common.saved": "Saved.",
    "common.unknown": "Unknown",
    "date_style.locale": "Locale default",
    "date_style.iso": "ISO 8601",
    "theme.system": "System",
    "theme.midnight": "Midnight",
    "theme.paper": "Paper",
    "theme.daybreak": "Daybreak",
    "ext.observatory.page.events": "Observatory",
    "ext.observatory.panel.timeline": "Event Timeline",
    "ext.observatory.theme.daybreak": "Daybreak",
    "ext.observatory.command.open": "Open Observatory"
  },
};

export function getCoreMessages(locale: string): TranslationBundle {
  return CORE_MESSAGES[locale] ?? CORE_MESSAGES["en-US"];
}

export function resolveLocalizedText(
  text: LocalizedText,
  locale: string,
  bundles: TranslationBundle[],
): string {
  for (const bundle of bundles) {
    if (bundle[text.key]) {
      return bundle[text.key];
    }
  }
  return getCoreMessages(locale)[text.key] ?? text.fallback;
}

export function translate(
  locale: string,
  key: string,
  fallback: string,
  bundles: TranslationBundle[] = [],
): string {
  return resolveLocalizedText({ key, fallback }, locale, bundles);
}

export function formatDateTime(value: string | number | Date, locale: string, timeZone?: string) {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(value));
}

export function formatDate(value: string | number | Date, locale: string, timeZone?: string) {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeZone,
  }).format(new Date(value));
}

export function formatTime(value: string | number | Date, locale: string, timeZone?: string) {
  return new Intl.DateTimeFormat(locale, {
    timeStyle: "short",
    timeZone,
  }).format(new Date(value));
}
