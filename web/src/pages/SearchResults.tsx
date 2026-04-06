import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api, type SearchResult } from "../api/client";
import TemplateCard from "../components/TemplateCard";

export default function SearchResults() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);

  const q = searchParams.get("q") || "";
  const tags = searchParams.get("tags") || "";
  const category = searchParams.get("category") || "";

  const [searchInput, setSearchInput] = useState(q);
  const [tagInput, setTagInput] = useState(tags);
  const [categoryInput, setCategoryInput] = useState(category);

  useEffect(() => {
    setLoading(true);
    api
      .search({ q: q || undefined, tags: tags || undefined, category: category || undefined, limit: 50 })
      .then((r) => setResults(r.results))
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [q, tags, category]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    const params: Record<string, string> = {};
    if (searchInput.trim()) params.q = searchInput.trim();
    if (tagInput.trim()) params.tags = tagInput.trim();
    if (categoryInput.trim()) params.category = categoryInput.trim();
    setSearchParams(params);
  };

  return (
    <div>
      <h2 style={{ marginBottom: "1.5rem" }}>Browse Templates</h2>

      <form
        onSubmit={handleSearch}
        style={{
          display: "flex",
          gap: "0.75rem",
          marginBottom: "2rem",
          flexWrap: "wrap",
        }}
      >
        <input
          type="search"
          placeholder="Search by name or description..."
          value={searchInput}
          onChange={(e) => setSearchInput(e.target.value)}
          style={{ flex: 2, minWidth: "200px" }}
        />
        <input
          type="text"
          placeholder="Tags (comma-separated)"
          value={tagInput}
          onChange={(e) => setTagInput(e.target.value)}
          style={{ flex: 1, minWidth: "150px" }}
        />
        <input
          type="text"
          placeholder="Category"
          value={categoryInput}
          onChange={(e) => setCategoryInput(e.target.value)}
          style={{ flex: 1, minWidth: "120px" }}
        />
        <button type="submit" className="btn btn-primary">
          Search
        </button>
      </form>

      {loading && <p style={{ color: "var(--text-muted)" }}>Searching...</p>}

      {!loading && results.length === 0 && (
        <p style={{ color: "var(--text-muted)" }}>
          {q || tags || category ? "No templates found matching your criteria." : "No templates indexed yet."}
        </p>
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
