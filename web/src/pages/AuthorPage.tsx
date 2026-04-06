import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api, type SearchResult } from "../api/client";
import TemplateCard from "../components/TemplateCard";

export default function AuthorPage() {
  const { pubkey } = useParams<{ pubkey: string }>();
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!pubkey) return;
    setLoading(true);
    api
      .search({ author: pubkey, limit: 100 })
      .then((r) => setResults(r.results))
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [pubkey]);

  // Use the friendly name from the first result that has one
  const friendlyName = results.find((r) => r.author_friendly_name)?.author_friendly_name;

  return (
    <div>
      <h1 style={{ fontSize: "2.5rem", marginBottom: "0.5rem" }}>
        {friendlyName || "Author"}
      </h1>
      <p style={{ color: "var(--text-muted)", marginBottom: "2rem" }}>
        <code style={{ fontSize: "0.8rem" }}>{pubkey}</code>
      </p>

      {loading && <p style={{ color: "var(--text-muted)" }}>Loading...</p>}

      {!loading && results.length === 0 && (
        <p style={{ color: "var(--text-muted)" }}>No templates found for this author.</p>
      )}

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))",
          gap: "1rem",
        }}
      >
        {results.map((r) => (
          <TemplateCard key={r.template_address} data={r} />
        ))}
      </div>
    </div>
  );
}
