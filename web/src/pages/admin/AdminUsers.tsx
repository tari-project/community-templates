import { useEffect, useState } from "react";
import { api, type AdminUser } from "../../api/client";

export default function AdminUsers() {
  const [admins, setAdmins] = useState<AdminUser[]>([]);
  const [newUsername, setNewUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [changingPasswordFor, setChangingPasswordFor] = useState<number | null>(null);
  const [newPw, setNewPw] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = () => {
    api.admin.listAdmins().then(setAdmins).catch(console.error);
  };

  useEffect(load, []);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await api.admin.createAdmin(newUsername, newPassword);
      setNewUsername("");
      setNewPassword("");
      load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create admin");
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm("Delete this admin?")) return;
    try {
      await api.admin.deleteAdmin(id);
      load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleChangePassword = async (id: number) => {
    try {
      await api.admin.changePassword(id, newPw);
      setChangingPasswordFor(null);
      setNewPw("");
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div>
      <h2 style={{ marginBottom: "1.5rem" }}>Admin Users</h2>

      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", marginBottom: "2rem" }}>
        {admins.map((a) => (
          <div
            key={a.id}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "0.75rem 1rem",
              background: "var(--grad-accent)",
              border: "1px solid var(--color-purple-dim)",
              borderRadius: "var(--radius)",
              flexWrap: "wrap",
              gap: "0.5rem",
            }}
          >
            <div>
              <strong>{a.username}</strong>
              <span style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginLeft: "0.75rem" }}>
                Created {new Date(a.created_at).toLocaleDateString()}
              </span>
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button
                className="btn btn-outline"
                onClick={() => setChangingPasswordFor(changingPasswordFor === a.id ? null : a.id)}
                style={{ padding: "0.3rem 0.8rem", fontSize: "0.8rem" }}
              >
                Change Password
              </button>
              <button
                className="btn btn-danger"
                onClick={() => handleDelete(a.id)}
                style={{ padding: "0.3rem 0.8rem", fontSize: "0.8rem" }}
              >
                Delete
              </button>
            </div>
            {changingPasswordFor === a.id && (
              <div style={{ width: "100%", display: "flex", gap: "0.5rem", marginTop: "0.5rem" }}>
                <input
                  type="password"
                  placeholder="New password"
                  value={newPw}
                  onChange={(e) => setNewPw(e.target.value)}
                  style={{ flex: 1 }}
                />
                <button
                  className="btn btn-primary"
                  onClick={() => handleChangePassword(a.id)}
                  style={{ padding: "0.3rem 0.8rem", fontSize: "0.8rem" }}
                >
                  Save
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      <h3 style={{ marginBottom: "1rem" }}>Add Admin</h3>
      <form onSubmit={handleCreate} style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
        <input
          type="text"
          placeholder="Username"
          value={newUsername}
          onChange={(e) => setNewUsername(e.target.value)}
          required
          style={{ flex: 1, minWidth: "150px" }}
        />
        <input
          type="password"
          placeholder="Password"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
          required
          style={{ flex: 1, minWidth: "150px" }}
        />
        <button type="submit" className="btn btn-primary">
          Create Admin
        </button>
      </form>
      {error && <p style={{ color: "#e53e3e", fontSize: "0.85rem", marginTop: "0.5rem" }}>{error}</p>}
    </div>
  );
}
