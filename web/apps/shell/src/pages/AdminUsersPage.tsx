import { useEffect, useState, type FormEvent } from "react";

import {
  adminCreateUser,
  adminDeleteUser,
  adminListUsers,
  adminResetPassword,
  adminUpdateUser,
  type User,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function AdminUsersPage() {
  const [users, setUsers] = useState<User[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [newUsername, setNewUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [newDisplay, setNewDisplay] = useState("");
  const [newRole, setNewRole] = useState<"user" | "admin">("user");
  const { formatDate, formatDateTime } = useUiHelpers();

  async function refresh() {
    setError(null);
    try {
      setUsers(await adminListUsers());
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await adminCreateUser({
        username: newUsername,
        password: newPassword,
        display_name: newDisplay || undefined,
        role: newRole,
      });
      setNewUsername("");
      setNewPassword("");
      setNewDisplay("");
      setNewRole("user");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleToggleRole(user: User) {
    const next = user.role === "admin" ? "user" : "admin";
    await adminUpdateUser(user.id, { role: next });
    await refresh();
  }

  async function handleDelete(user: User) {
    if (!confirm(`Delete user ${user.username}?`)) return;
    await adminDeleteUser(user.id);
    await refresh();
  }

  async function handleResetPassword(user: User) {
    const pwd = prompt(`New password for ${user.username}:`);
    if (!pwd) return;
    await adminResetPassword(user.id, pwd);
    alert("Password reset.");
  }

  return (
    <div className="page">
      <h1>Users</h1>
      {error && <div className="error">{error}</div>}

      <section>
        <h3>Create user</h3>
        <form onSubmit={handleCreate} className="inline-form">
          <input
            value={newUsername}
            onChange={(e) => setNewUsername(e.target.value)}
            placeholder="username"
            required
          />
          <input
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            placeholder="password"
            type="password"
            required
            minLength={6}
          />
          <input
            value={newDisplay}
            onChange={(e) => setNewDisplay(e.target.value)}
            placeholder="display name"
          />
          <select value={newRole} onChange={(e) => setNewRole(e.target.value as never)}>
            <option value="user">user</option>
            <option value="admin">admin</option>
          </select>
          <button type="submit" disabled={busy}>
            Create
          </button>
        </form>
      </section>

      <table className="table">
        <thead>
          <tr>
            <th>Username</th>
            <th>Display</th>
            <th>Role</th>
            <th>Created</th>
            <th>Last login</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {users.map((u) => (
            <tr key={u.id}>
              <td>
                <strong>{u.username}</strong>
                <br />
                <code>{u.id.slice(0, 16)}</code>
              </td>
              <td>{u.display_name ?? "—"}</td>
              <td>
                <span className={`pill pill--${u.role}`}>{u.role}</span>
              </td>
              <td>{formatDate(u.created_at)}</td>
              <td>{u.last_login_at ? formatDateTime(u.last_login_at) : "—"}</td>
              <td className="row-actions">
                <button onClick={() => handleToggleRole(u)}>
                  {u.role === "admin" ? "Demote" : "Promote"}
                </button>
                <button onClick={() => handleResetPassword(u)}>Reset password</button>
                <button onClick={() => handleDelete(u)} className="danger">
                  Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
