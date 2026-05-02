import { useEffect, useState } from "react";
import { api, type StatsResponse } from "../../api/client";

export default function Dashboard() {
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [reindexing, setReindexing] = useState(false);
  const [reindexMessage, setReindexMessage] = useState<string | null>(null);
  const [reindexError, setReindexError] = useState<string | null>(null);

  function refreshStats() {
    api.admin.getStats().then(setStats).catch(console.error);
  }

  useEffect(() => {
    refreshStats();
  }, []);

  async function handleReindex() {
    const confirmation = window.prompt(
      "This will DELETE all indexed templates and metadata, then re-pull everything from the indexer.\n\n" +
        "Admin users will be preserved.\n\n" +
        'Type "REINDEX" to confirm.',
    );
    if (confirmation !== "REINDEX") {
      return;
    }
    setReindexing(true);
    setReindexMessage(null);
    setReindexError(null);
    try {
      const result = await api.admin.reindex();
      setReindexMessage(
        `Wiped ${result.deleted_templates} templates and ${result.deleted_metadata} metadata rows. Resync started in the background.`,
      );
      // Stats will hit zero immediately and climb back as the sync loop runs.
      refreshStats();
    } catch (e) {
      setReindexError(e instanceof Error ? e.message : String(e));
    } finally {
      setReindexing(false);
    }
  }

  return (
    <div>
      <h2 style={{ marginBottom: "2rem" }}>Dashboard</h2>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
          gap: "1rem",
          marginBottom: "2rem",
        }}
      >
        <StatCard label="Templates" value={stats?.total_templates} />
        <StatCard label="With Metadata" value={stats?.with_metadata} />
        <StatCard label="With Definition" value={stats?.with_definition} />
        <StatCard label="Featured" value={stats?.featured} />
        <StatCard label="Blacklisted" value={stats?.blacklisted} />
      </div>

      <section
        style={{
          marginTop: "3rem",
          padding: "1.5rem",
          border: "1px solid #b33",
          borderRadius: "var(--radius)",
          background: "rgba(179, 51, 51, 0.05)",
        }}
      >
        <h3 style={{ marginTop: 0, color: "#b33" }}>Danger Zone</h3>
        <p style={{ color: "var(--text-muted)", fontSize: "0.9rem" }}>
          Wipe all indexed templates and metadata, then trigger a full resync from
          the indexer. Admin users and credentials are preserved. Useful if the
          local index drifts from the indexer or after schema changes.
        </p>
        <button
          type="button"
          onClick={handleReindex}
          disabled={reindexing}
          style={{
            padding: "0.6rem 1.2rem",
            background: reindexing ? "#888" : "#b33",
            color: "white",
            border: "none",
            borderRadius: "var(--radius)",
            cursor: reindexing ? "wait" : "pointer",
            fontWeight: 600,
          }}
        >
          {reindexing ? "Reindexing..." : "Wipe & Reindex Database"}
        </button>
        {reindexMessage && (
          <p style={{ marginTop: "1rem", color: "#2a7" }}>{reindexMessage}</p>
        )}
        {reindexError && (
          <p style={{ marginTop: "1rem", color: "#b33" }}>Error: {reindexError}</p>
        )}
      </section>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value?: number }) {
  return (
    <div
      style={{
        padding: "1.25rem",
        background: "var(--grad-accent)",
        border: "1px solid var(--color-purple-dim)",
        borderRadius: "var(--radius)",
        textAlign: "center",
      }}
    >
      <div
        style={{
          fontSize: "2rem",
          fontWeight: 700,
          fontFamily: "var(--font-heading)",
          color: "var(--color-purple)",
        }}
      >
        {value ?? "-"}
      </div>
      <div style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginTop: "0.25rem" }}>
        {label}
      </div>
    </div>
  );
}
