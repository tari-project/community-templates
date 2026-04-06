import { Link, useNavigate } from "react-router-dom";
import { useState } from "react";

export default function Header() {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (query.trim()) {
      navigate(`/search?q=${encodeURIComponent(query.trim())}`);
    }
  };

  return (
    <header
      style={{
        background: "rgba(4, 7, 35, 0.85)",
        backdropFilter: "blur(40px)",
        borderBottom: "1px solid rgba(149, 125, 232, 0.1)",
        position: "sticky",
        top: 0,
        zIndex: 100,
      }}
    >
      <div
        className="container"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          height: "4rem",
          gap: "1.5rem",
        }}
      >
        <Link
          to="/"
          style={{
            fontFamily: "var(--font-heading)",
            fontSize: "1.4rem",
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.05rem",
            color: "var(--color-ink)",
            textDecoration: "none",
            whiteSpace: "nowrap",
          }}
        >
          Ootle Templates
        </Link>

        <form onSubmit={handleSearch} style={{ flex: 1, maxWidth: "400px" }}>
          <input
            type="search"
            placeholder="Search templates..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{ width: "100%" }}
          />
        </form>

        <nav style={{ display: "flex", gap: "1rem", alignItems: "center" }}>
          <Link to="/search" style={{ color: "var(--text-muted)", fontSize: "0.9rem" }}>
            Browse
          </Link>
        </nav>
      </div>
    </header>
  );
}
