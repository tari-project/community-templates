import { Link, useLocation, useNavigate } from "react-router-dom";
import { logout } from "../api/client";

const links = [
  { to: "/admin", label: "Dashboard" },
  { to: "/admin/featured", label: "Featured" },
  { to: "/admin/blacklist", label: "Blacklist" },
  { to: "/admin/users", label: "Users" },
];

export default function AdminNav() {
  const location = useLocation();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
    navigate("/admin/login");
  };

  return (
    <nav
      style={{
        display: "flex",
        alignItems: "center",
        gap: "0.25rem",
        marginBottom: "2rem",
        padding: "0.5rem",
        background: "var(--grad-accent)",
        border: "1px solid var(--color-purple-dim)",
        borderRadius: "var(--radius)",
        flexWrap: "wrap",
      }}
    >
      {links.map(({ to, label }) => {
        const active = location.pathname === to;
        return (
          <Link
            key={to}
            to={to}
            style={{
              padding: "0.4rem 0.8rem",
              borderRadius: "4px",
              fontSize: "0.85rem",
              fontWeight: 600,
              textDecoration: "none",
              color: active ? "#fff" : "var(--text-muted)",
              background: active ? "var(--color-purple)" : "transparent",
            }}
          >
            {label}
          </Link>
        );
      })}
      <div style={{ flex: 1 }} />
      <button
        onClick={handleLogout}
        className="btn btn-outline"
        style={{ padding: "0.3rem 0.8rem", fontSize: "0.8rem" }}
      >
        Logout
      </button>
    </nav>
  );
}
