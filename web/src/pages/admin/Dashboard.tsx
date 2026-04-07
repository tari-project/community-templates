import { useEffect, useState } from "react";
import { api, type StatsResponse } from "../../api/client";

export default function Dashboard() {
  const [stats, setStats] = useState<StatsResponse | null>(null);

  useEffect(() => {
    api.admin.getStats().then(setStats).catch(console.error);
  }, []);

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
