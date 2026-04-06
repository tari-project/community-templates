import { useEffect, useState } from "react";
import { api, type AdminTemplate } from "../../api/client";

export default function BlacklistManager() {
  const [templates, setTemplates] = useState<AdminTemplate[]>([]);
  const [loading, setLoading] = useState(true);

  const load = () => {
    setLoading(true);
    api.admin
      .listTemplates(200)
      .then(setTemplates)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  useEffect(load, []);

  const toggleBlacklist = async (addr: string, current: boolean) => {
    try {
      await api.admin.setBlacklisted(addr, !current);
      load();
    } catch (e) {
      console.error(e);
    }
  };

  const blacklisted = templates.filter((t) => t.is_blacklisted);
  const active = templates.filter((t) => !t.is_blacklisted);

  return (
    <div>
      <h2 style={{ marginBottom: "1.5rem" }}>Blacklist Management</h2>

      {loading && <p style={{ color: "var(--text-muted)" }}>Loading...</p>}

      {blacklisted.length > 0 && (
        <>
          <h3 style={{ marginBottom: "1rem", fontSize: "1rem" }}>Blacklisted</h3>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", marginBottom: "2rem" }}>
            {blacklisted.map((t) => (
              <TemplateRow key={t.template_address} t={t} onToggle={toggleBlacklist} label="Unblock" />
            ))}
          </div>
        </>
      )}

      <h3 style={{ marginBottom: "1rem", fontSize: "1rem" }}>Active Templates</h3>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
        {active.map((t) => (
          <TemplateRow key={t.template_address} t={t} onToggle={toggleBlacklist} label="Blacklist" />
        ))}
      </div>
    </div>
  );
}

function TemplateRow({
  t,
  onToggle,
  label,
}: {
  t: AdminTemplate;
  onToggle: (addr: string, current: boolean) => void;
  label: string;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0.75rem 1rem",
        background: "var(--grad-accent)",
        border: `1px solid ${t.is_blacklisted ? "rgba(229, 62, 62, 0.3)" : "var(--color-purple-dim)"}`,
        borderRadius: "var(--radius)",
      }}
    >
      <div>
        <strong>{t.template_name}</strong>
        <span style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginLeft: "0.75rem" }}>
          {t.template_address.slice(0, 12)}...
        </span>
      </div>
      <button
        className={`btn ${t.is_blacklisted ? "btn-outline" : "btn-danger"}`}
        onClick={() => onToggle(t.template_address, t.is_blacklisted)}
        style={{ padding: "0.3rem 0.8rem", fontSize: "0.8rem" }}
      >
        {label}
      </button>
    </div>
  );
}
