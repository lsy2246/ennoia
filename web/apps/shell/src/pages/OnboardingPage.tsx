import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState, type FormEvent } from "react";

import { completeBootstrap } from "@ennoia/api-client";
import { useAuthStore } from "@/stores/auth";

const AUTH_MODE_OPTIONS = [
  {
    value: "session",
    title: "Session",
    hint: "推荐本地体验使用，浏览器登录后自动携带会话。",
  },
  {
    value: "jwt",
    title: "JWT",
    hint: "适合对接前后端分离调用，便于显式管理 token。",
  },
  {
    value: "none",
    title: "None",
    hint: "不开启认证，仅适合本地临时演示或纯开发环境。",
  },
] as const;

export function OnboardingPage() {
  const navigate = useNavigate();
  const bootstrap = useAuthStore((s) => s.bootstrap);
  const hydrate = useAuthStore((s) => s.hydrate);
  const login = useAuthStore((s) => s.login);

  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [displayName, setDisplayName] = useState("Administrator");
  const [authMode, setAuthMode] = useState<"session" | "jwt" | "none">("session");
  const [allowReg, setAllowReg] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const selectedAuthMode = AUTH_MODE_OPTIONS.find((item) => item.value === authMode);

  useEffect(() => {
    if (bootstrap?.completed) {
      navigate({ to: "/login" });
    }
  }, [bootstrap, navigate]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (password !== confirmPassword) {
      setError("两次输入的管理员密码不一致。");
      return;
    }
    setBusy(true);
    try {
      await completeBootstrap({
        admin_username: username,
        admin_password: password,
        admin_display_name: displayName || undefined,
        auth_mode: authMode,
        allow_registration: allowReg,
      });
      if (authMode !== "none") {
        await login(username, password);
      }
      await hydrate();
      navigate({ to: "/" });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page page--centered onboarding-page">
      <section className="onboarding-hero">
        <span className="onboarding-hero__eyebrow">首次启动引导</span>
        <h1>先完成一次初始化，再进入 Ennoia 工作台。</h1>
        <p className="onboarding-hero__lead">
          当前检测到这是一套尚未完成引导的运行环境。下面会帮你创建第一个管理员账号，并确定登录方式。
        </p>

        <div className="onboarding-checklist">
          <div className="onboarding-checklist__item">
            <strong>1. 创建管理员</strong>
            <span>设置第一个可登录后台的管理员用户名、显示名和密码。</span>
          </div>
          <div className="onboarding-checklist__item">
            <strong>2. 选择认证模式</strong>
            <span>默认推荐 Session，本地浏览器使用最顺手。</span>
          </div>
          <div className="onboarding-checklist__item">
            <strong>3. 完成初始化</strong>
            <span>提交后会自动登录，并进入主工作台首页。</span>
          </div>
        </div>

        <div className="onboarding-notes">
          <div className="onboarding-note">
            <strong>当前访问入口</strong>
            <span>
              你现在访问的是前端主壳 <code>http://127.0.0.1:5173</code>，后端 API 在{" "}
              <code>http://127.0.0.1:3710</code>。
            </span>
          </div>
          <div className="onboarding-note">
            <strong>推荐配置</strong>
            <span>
              本地测试建议管理员用户名使用 <code>admin</code>，认证模式选择{" "}
              <code>Session</code>。
            </span>
          </div>
        </div>
      </section>

      <form onSubmit={handleSubmit} className="auth-card auth-card--wide onboarding-card">
        <div className="onboarding-card__header">
          <h2>初始化信息</h2>
          <p>这些信息会写入当前运行目录，并作为这套 Ennoia 实例的首批系统配置。</p>
        </div>

        <div className="form-stack">
          <div className="form-row">
            <label>
              管理员用户名
              <input value={username} onChange={(e) => setUsername(e.target.value)} required />
            </label>
            <label>
              显示名称
              <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
            </label>
          </div>

          <div className="form-row">
            <label>
              管理员密码
              <input
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                type="password"
                required
                minLength={6}
                autoComplete="new-password"
              />
            </label>
            <label>
              确认密码
              <input
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                type="password"
                required
                minLength={6}
                autoComplete="new-password"
              />
            </label>
          </div>
        </div>

        <div className="onboarding-section">
          <div className="onboarding-section__title">
            <h3>认证方式</h3>
            <p>选择这套实例后续默认采用的登录模式。</p>
          </div>
          <div className="onboarding-auth-grid">
            {AUTH_MODE_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                className={`onboarding-auth-option${
                  authMode === option.value ? " onboarding-auth-option--active" : ""
                }`}
                onClick={() => setAuthMode(option.value)}
              >
                <strong>{option.title}</strong>
                <span>{option.hint}</span>
              </button>
            ))}
          </div>
          <div className="onboarding-auth-summary">
            当前选择：<strong>{selectedAuthMode?.title}</strong>
            <span>{selectedAuthMode?.hint}</span>
          </div>
        </div>

        <div className="onboarding-section">
          <label className="auth-card__checkbox onboarding-checkbox">
            <input
              type="checkbox"
              checked={allowReg}
              onChange={(e) => setAllowReg(e.target.checked)}
            />
            允许普通用户自助注册
          </label>
          <p className="onboarding-checkbox__hint">
            若不勾选，后续只有管理员可以在后台创建新用户，更适合私有部署或本地测试。
          </p>
        </div>

        {error && <div className="auth-card__error">{error}</div>}

        <div className="onboarding-actions">
          <button type="submit" disabled={busy}>
            {busy ? "正在初始化…" : "完成初始化并进入工作台"}
          </button>
        </div>
      </form>
    </div>
  );
}
