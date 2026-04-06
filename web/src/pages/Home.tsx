import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type TemplateResponse } from "../api/client";
import TemplateCard from "../components/TemplateCard";

export default function Home() {
  const [featured, setFeatured] = useState<TemplateResponse[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api
      .getFeatured()
      .then(setFeatured)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return (
    <div>
      <section style={{ textAlign: "center", padding: "4rem 0 3rem" }}>
        <h1>Community Templates</h1>
        <p
          style={{
            color: "var(--text-muted)",
            fontSize: "1.1rem",
            marginTop: "1rem",
            maxWidth: "600px",
            marginLeft: "auto",
            marginRight: "auto",
          }}
        >
          Discover, search, and explore templates built by the Ootle community.
        </p>
        <div style={{ marginTop: "2rem" }}>
          <Link to="/search" className="btn btn-primary" style={{ padding: "0.8rem 2rem", fontSize: "1rem" }}>
            Browse All Templates
          </Link>
        </div>
      </section>

      {featured.length > 0 && (
        <section>
          <h2 style={{ marginBottom: "1.5rem" }}>Featured</h2>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))",
              gap: "1rem",
            }}
          >
            {featured.map((t) => (
              <TemplateCard key={t.template_address} data={t} />
            ))}
          </div>
        </section>
      )}

      {loading && (
        <p style={{ textAlign: "center", color: "var(--text-muted)" }}>
          Loading...
        </p>
      )}
    </div>
  );
}
