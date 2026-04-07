import { useEffect, useState } from "react";
import { api, type AdminTemplate } from "../../api/client";
import SafeImage from "../../components/SafeImage";

export default function FeaturedManager() {
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

  const toggleFeatured = async (addr: string, current: boolean) => {
    try {
      await api.admin.setFeatured(addr, !current);
      load();
    } catch (e) {
      console.error(e);
    }
  };

  const featured = templates.filter((t) => t.is_featured);
  const others = templates.filter((t) => !t.is_featured && !t.is_blacklisted);

  return (
    <div>
      <h2 style={{ marginBottom: "1rem" }}>Featured Templates</h2>
      <p style={{ color: "var(--text-muted)", fontSize: "0.85rem", marginBottom: "1.5rem" }}>
        Featured templates have their logo displayed publicly on the site.
        Review the logo URL before featuring a template.
      </p>

      {loading && <p style={{ color: "var(--text-muted)" }}>Loading...</p>}

      <h3 style={{ marginBottom: "1rem", fontSize: "1rem" }}>Currently Featured</h3>
      {featured.length === 0 && (
        <p style={{ color: "var(--text-muted)", marginBottom: "1.5rem" }}>No templates featured yet.</p>
      )}
      <TemplateList templates={featured} onToggle={toggleFeatured} actionLabel="Unfeature" />

      <h3 style={{ marginTop: "2rem", marginBottom: "1rem", fontSize: "1rem" }}>All Templates</h3>
      <TemplateList templates={others} onToggle={toggleFeatured} actionLabel="Feature" />
    </div>
  );
}

function TemplateList({
  templates,
  onToggle,
  actionLabel,
}: {
  templates: AdminTemplate[];
  onToggle: (addr: string, current: boolean) => void;
  actionLabel: string;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      {templates.map((t) => (
        <div
          key={t.template_address}
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
            padding: "0.75rem 1rem",
            background: "var(--grad-accent)",
            border: "1px solid var(--color-purple-dim)",
            borderRadius: "var(--radius)",
          }}
        >
          <SafeImage
            url={t.logo_url}
            alt={t.template_name}
            size={32}
            trusted
          />
          <div style={{ flex: 1, minWidth: 0 }}>
            <strong>{t.template_name}</strong>
            <span style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginLeft: "0.75rem" }}>
              {t.template_address.slice(0, 12)}...
            </span>
          </div>
          <button
            className={`btn ${t.is_featured ? "btn-outline" : "btn-primary"}`}
            onClick={() => onToggle(t.template_address, t.is_featured)}
            style={{ padding: "0.3rem 0.8rem", fontSize: "0.8rem" }}
          >
            {actionLabel}
          </button>
        </div>
      ))}
    </div>
  );
}
