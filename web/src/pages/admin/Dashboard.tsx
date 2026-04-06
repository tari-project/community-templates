import { Link } from "react-router-dom";

export default function Dashboard() {
  return (
    <div>
      <h2 style={{ marginBottom: "2rem" }}>Admin Dashboard</h2>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
          gap: "1rem",
        }}
      >
        <AdminLink to="/admin/featured" title="Featured Templates" description="Manage which templates are featured on the homepage" />
        <AdminLink to="/admin/blacklist" title="Blacklist" description="Block templates from appearing in search results" />
        <AdminLink to="/admin/users" title="Admin Users" description="Manage admin accounts and passwords" />
      </div>
    </div>
  );
}

function AdminLink({ to, title, description }: { to: string; title: string; description: string }) {
  return (
    <Link
      to={to}
      style={{
        display: "block",
        padding: "1.5rem",
        background: "var(--grad-accent)",
        border: "1px solid var(--color-purple-dim)",
        borderRadius: "var(--radius)",
        textDecoration: "none",
        color: "inherit",
      }}
    >
      <h3 style={{ marginBottom: "0.5rem" }}>{title}</h3>
      <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>{description}</p>
    </Link>
  );
}
