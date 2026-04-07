import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api, type CategoryCount, type SearchResult, type TagCount } from "../api/client";
import CategorySelect from "../components/CategorySelect";
import TagInput from "../components/TagInput";
import TemplateCard from "../components/TemplateCard";

export default function SearchResults() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [tagSuggestions, setTagSuggestions] = useState<TagCount[]>([]);
  const [categorySuggestions, setCategorySuggestions] = useState<CategoryCount[]>([]);

  const q = searchParams.get("q") || "";
  const tags = searchParams.get("tags") || "";
  const category = searchParams.get("category") || "";

  const [searchInput, setSearchInput] = useState(q);
  const [selectedTags, setSelectedTags] = useState<string[]>(
    tags ? tags.split(",").map((t) => t.trim()).filter(Boolean) : [],
  );
  const [categoryInput, setCategoryInput] = useState(category);

  // Sync inputs with URL params (e.g. browser back/forward)
  useEffect(() => {
    setSearchInput(q);
    setSelectedTags(tags ? tags.split(",").map((t) => t.trim()).filter(Boolean) : []);
    setCategoryInput(category);
  }, [q, tags, category]);

  // Fetch suggestions once
  useEffect(() => {
    api.getTags().then(setTagSuggestions).catch(console.error);
    api.getCategories().then(setCategorySuggestions).catch(console.error);
  }, []);

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
    if (selectedTags.length > 0) params.tags = selectedTags.join(",");
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
          alignItems: "flex-start",
        }}
      >
        <input
          type="search"
          placeholder="Search by name or description..."
          value={searchInput}
          onChange={(e) => setSearchInput(e.target.value)}
          style={{ flex: 2, minWidth: "200px" }}
        />
        <TagInput
          value={selectedTags}
          onChange={setSelectedTags}
          suggestions={tagSuggestions}
          placeholder="Filter by tags..."
        />
        <CategorySelect
          value={categoryInput}
          onChange={setCategoryInput}
          suggestions={categorySuggestions}
          placeholder="Category..."
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
