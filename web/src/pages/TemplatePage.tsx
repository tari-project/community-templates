import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, type TemplateResponse } from "../api/client";
import SafeImage from "../components/SafeImage";
import FunctionSignature from "../components/FunctionSignature";

export default function TemplatePage() {
  const { addr } = useParams<{ addr: string }>();
  const [template, setTemplate] = useState<TemplateResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!addr) return;
    setLoading(true);
    api
      .getTemplate(addr)
      .then(setTemplate)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [addr]);

  if (loading) return <p style={{ color: "var(--text-muted)" }}>Loading...</p>;
  if (error) return <p style={{ color: "#e53e3e" }}>Error: {error}</p>;
  if (!template) return <p>Template not found.</p>;

  const meta = template.metadata;
  const def = template.definition?.V1;
  const truncatedAuthor = template.author_public_key.slice(0, 16) + "...";

  return (
    <div>
      {/* Header */}
      <div style={{ display: "flex", gap: "1.5rem", alignItems: "flex-start", marginBottom: "2rem" }}>
        <SafeImage
          url={meta?.logo_url ?? null}
          alt={meta?.name || template.template_name}
          size={80}
          trusted={template.is_featured}
        />
        <div>
          <h1 style={{ fontSize: "2.5rem" }}>{meta?.name || template.template_name}</h1>
          <div
            style={{
              display: "flex",
              gap: "1rem",
              alignItems: "center",
              marginTop: "0.5rem",
              flexWrap: "wrap",
            }}
          >
            <Link
              to={`/authors/${template.author_public_key}`}
              style={{ color: "var(--text-muted)", fontSize: "0.85rem", textDecoration: "none" }}
            >
              by {template.author_friendly_name || <code>{truncatedAuthor}</code>}
            </Link>
            <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
              Epoch {template.at_epoch}
            </span>
            {template.code_size && (
              <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
                {(template.code_size / 1024).toFixed(1)} KB
              </span>
            )}
            {meta?.category && <span className="badge badge--purple">{meta.category}</span>}
            {meta?.version && <span className="badge badge--muted">v{meta.version}</span>}
          </div>
        </div>
      </div>

      {/* Metadata */}
      {meta ? (
        <section style={{ marginBottom: "2rem" }}>
          {meta.description && (
            <p style={{ fontSize: "1rem", lineHeight: 1.7, marginBottom: "1rem" }}>
              {meta.description}
            </p>
          )}

          {meta.tags.length > 0 && (
            <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", marginBottom: "1rem" }}>
              {meta.tags.map((tag) => (
                <span key={tag} className="badge badge--muted">{tag}</span>
              ))}
            </div>
          )}

          <div style={{ display: "flex", gap: "1.5rem", flexWrap: "wrap", fontSize: "0.9rem" }}>
            {meta.repository && (
              <a href={meta.repository} target="_blank" rel="noopener noreferrer">
                Repository
              </a>
            )}
            {meta.documentation && (
              <a href={meta.documentation} target="_blank" rel="noopener noreferrer">
                Documentation
              </a>
            )}
            {meta.homepage && (
              <a href={meta.homepage} target="_blank" rel="noopener noreferrer">
                Homepage
              </a>
            )}
            {meta.license && (
              <span style={{ color: "var(--text-muted)" }}>License: {meta.license}</span>
            )}
          </div>

          {meta.commit_hash && (
            <div style={{ marginTop: "0.75rem", fontSize: "0.85rem", color: "var(--text-muted)" }}>
              Commit: <code style={{ fontSize: "0.8rem" }}>{meta.commit_hash}</code>
              {meta.repository && (
                <>
                  {" "}
                  <a
                    href={`${meta.repository.replace(/\.git$/, "")}/commit/${meta.commit_hash}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ fontSize: "0.8rem" }}
                  >
                    (view)
                  </a>
                </>
              )}
            </div>
          )}

          {meta.supersedes && (
            <div style={{ marginTop: "0.5rem", fontSize: "0.85rem" }}>
              Supersedes:{" "}
              <Link to={`/templates/${meta.supersedes}`} style={{ fontSize: "0.85rem" }}>
                <code style={{ fontSize: "0.8rem" }}>{meta.supersedes.slice(0, 16)}...</code>
              </Link>
            </div>
          )}
        </section>
      ) : (
        <section
          style={{
            marginBottom: "2rem",
            padding: "1.5rem",
            background: "var(--grad-accent)",
            border: "1px solid var(--color-purple-dim)",
            borderRadius: "var(--radius)",
          }}
        >
          <p style={{ color: "var(--text-muted)", display: "flex", alignItems: "flex-start", gap: "0.6rem" }}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ flexShrink: 0, marginTop: "0.15rem" }}>
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="16" x2="12" y2="12" />
              <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
            <span>
              No verified metadata available for this template. The template author can submit
              metadata to add a description, tags, links, and more. The recommended way to do this is
              using the{" "}
              <a href="https://tari-project.github.io/tari-cli/reference/template-publish/" target="_blank" rel="noopener noreferrer">
                Tari CLI
              </a>.
            </span>
          </p>
        </section>
      )}

      {/* Template Address */}
      <section style={{ marginBottom: "2rem" }}>
        <h3 style={{ marginBottom: "0.5rem" }}>Template Address</h3>
        <code style={{ fontSize: "0.8rem", wordBreak: "break-all" }}>
          {template.template_address}
        </code>
      </section>

      {/* Functions */}
      {def && def.functions.length > 0 && (
        <section>
          <h2 style={{ marginBottom: "1rem" }}>Functions & Methods</h2>
          {def.functions.map((f) => (
            <FunctionSignature key={f.name} func={f} />
          ))}
        </section>
      )}
    </div>
  );
}
