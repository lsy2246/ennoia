import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState, type FormEvent } from "react";

import { completeBootstrap } from "@ennoia/api-client";
import { useAuthStore } from "@/stores/auth";

export function OnboardingPage() {
  const navigate = useNavigate();
  const bootstrap = useAuthStore((s) => s.bootstrap);
  const hydrate = useAuthStore((s) => s.hydrate);
  const login = useAuthStore((s) => s.login);

  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("Administrator");
  const [authMode, setAuthMode] = useState<"session" | "jwt" | "none">("session");
  const [allowReg, setAllowReg] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (bootstrap?.completed) {
      navigate({ to: "/login" });
    }
  }, [bootstrap, navigate]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
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
    <div className="page page--centered">
      <form onSubmit={handleSubmit} className="auth-card auth-card--wide">
        <h1>Welcome to Ennoia</h1>
        <p className="auth-card__subtitle">
          Complete onboarding to create the first admin and configure authentication.
        </p>
        <label>
          Admin username
          <input value={username} onChange={(e) => setUsername(e.target.value)} required />
        </label>
        <label>
          Admin password
          <input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            type="password"
            required
            minLength={6}
          />
        </label>
        <label>
          Display name
          <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
        </label>
        <label>
          Auth mode
          <select value={authMode} onChange={(e) => setAuthMode(e.target.value as never)}>
            <option value="session">Session (recommended)</option>
            <option value="jwt">JWT</option>
            <option value="none">None (open access)</option>
          </select>
        </label>
        <label className="auth-card__checkbox">
          <input
            type="checkbox"
            checked={allowReg}
            onChange={(e) => setAllowReg(e.target.checked)}
          />
          Allow public self-registration
        </label>
        {error && <div className="auth-card__error">{error}</div>}
        <button type="submit" disabled={busy}>
          {busy ? "Provisioning…" : "Complete setup"}
        </button>
      </form>
    </div>
  );
}
