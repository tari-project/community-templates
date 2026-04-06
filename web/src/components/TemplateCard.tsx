import { Link } from "react-router-dom";
import type { SearchResult, TemplateResponse } from "../api/client";
import SafeImage from "./SafeImage";

type CardData = SearchResult | TemplateResponse;

function isSearchResult(d: CardData): d is SearchResult {
  return "has_metadata" in d;
}

export default function TemplateCard({ data }: { data: CardData }) {
  const addr = data.template_address;
  const name = isSearchResult(data)
    ? data.meta_name || data.template_name
    : data.metadata?.name || data.template_name;
  const description = isSearchResult(data)
    ? data.meta_description
    : data.metadata?.description;
  const tags = isSearchResult(data) ? data.meta_tags : data.metadata?.tags;
  const category = isSearchResult(data)
    ? data.meta_category
    : data.metadata?.category;
  const logoUrl = isSearchResult(data)
    ? data.meta_logo_url
    : data.metadata?.logo_url;

  return (
    <Link
      to={`/templates/${addr}`}
      style={{
        display: "block",
        background: "var(--grad-accent)",
        border: "1px solid var(--color-purple-dim)",
        borderRadius: "var(--radius)",
        padding: "1.25rem",
        textDecoration: "none",
        color: "inherit",
        transition: "border-color 0.2s",
      }}
      onMouseEnter={(e) =>
        (e.currentTarget.style.borderColor = "var(--color-purple)")
      }
      onMouseLeave={(e) =>
        (e.currentTarget.style.borderColor = "rgba(129, 88, 245, 0.15)")
      }
    >
      <div style={{ display: "flex", gap: "1rem", alignItems: "flex-start" }}>
        <SafeImage
          url={logoUrl ?? null}
          alt={name}
          size={48}
        />
        <div style={{ flex: 1, minWidth: 0 }}>
          <h3 style={{ fontSize: "1.1rem", marginBottom: "0.2rem" }}>{name}</h3>
          {data.author_friendly_name && (
            <p style={{ color: "var(--text-muted)", fontSize: "0.75rem", marginBottom: "0.2rem" }}>
              by {data.author_friendly_name}
            </p>
          )}
          {description && (
            <p
              style={{
                color: "var(--text-muted)",
                fontSize: "0.85rem",
                lineHeight: 1.5,
                overflow: "hidden",
                textOverflow: "ellipsis",
                display: "-webkit-box",
                WebkitLineClamp: 2,
                WebkitBoxOrient: "vertical",
              }}
            >
              {description}
            </p>
          )}
          <div
            style={{
              display: "flex",
              gap: "0.4rem",
              flexWrap: "wrap",
              marginTop: "0.6rem",
            }}
          >
            {category && <span className="badge badge--purple">{category}</span>}
            {tags?.slice(0, 3).map((tag) => (
              <span key={tag} className="badge badge--muted">
                {tag}
              </span>
            ))}
          </div>
        </div>
      </div>
    </Link>
  );
}
