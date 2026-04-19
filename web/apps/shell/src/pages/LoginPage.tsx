import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState, type FormEvent } from "react";

import { useAuthStore } from "@/stores/auth";
import { useUiHelpers } from "@/stores/ui";

export function LoginPage() {
  const navigate = useNavigate();
  const login = useAuthStore((s) => s.login);
  const hydrate = useAuthStore((s) => s.hydrate);
  const bootstrap = useAuthStore((s) => s.bootstrap);
  const user = useAuthStore((s) => s.user);
  const { t } = useUiHelpers();

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (bootstrap && !bootstrap.completed) {
      navigate({ to: "/onboarding" });
      return;
    }
    if (user) {
      navigate({ to: "/" });
    }
  }, [bootstrap, user, navigate]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await login(username, password);
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
      <form onSubmit={handleSubmit} className="auth-card">
        <h1>Ennoia</h1>
        <p className="auth-card__subtitle">{t("auth.login.subtitle", "Sign in to continue")}</p>
        <label>
          {t("auth.login.username", "Username")}
          <input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            autoComplete="username"
          />
        </label>
        <label>
          {t("auth.login.password", "Password")}
          <input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            type="password"
            autoComplete="current-password"
          />
        </label>
        {error && <div className="auth-card__error">{error}</div>}
        <button type="submit" disabled={busy}>
          {busy
            ? t("auth.login.submitting", "Signing in…")
            : t("auth.login.submit", "Sign in")}
        </button>
      </form>
    </div>
  );
}
