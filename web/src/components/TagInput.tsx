import { useEffect, useRef, useState } from "react";
import type { TagCount } from "../api/client";

interface TagInputProps {
  value: string[];
  onChange: (tags: string[]) => void;
  suggestions: TagCount[];
  placeholder?: string;
}

export default function TagInput({ value, onChange, suggestions, placeholder = "Add tags..." }: TagInputProps) {
  const [input, setInput] = useState("");
  const [open, setOpen] = useState(false);
  const [highlightIdx, setHighlightIdx] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const filtered = suggestions.filter(
    (s) => !value.includes(s.tag) && s.tag.toLowerCase().includes(input.toLowerCase()),
  );

  const addTag = (tag: string) => {
    const t = tag.trim().toLowerCase();
    if (t && !value.includes(t)) {
      onChange([...value, t]);
    }
    setInput("");
    setHighlightIdx(-1);
    inputRef.current?.focus();
  };

  const removeTag = (tag: string) => {
    onChange(value.filter((t) => t !== tag));
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.key === "Enter" || e.key === ",") && input.trim()) {
      e.preventDefault();
      if (highlightIdx >= 0 && highlightIdx < filtered.length) {
        addTag(filtered[highlightIdx].tag);
      } else {
        addTag(input);
      }
    } else if (e.key === "Backspace" && !input && value.length > 0) {
      removeTag(value[value.length - 1]);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlightIdx((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlightIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === "Escape") {
      setOpen(false);
      setHighlightIdx(-1);
    }
  };

  // Close dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const showDropdown = open && (filtered.length > 0 || input.trim());

  return (
    <div ref={containerRef} style={{ position: "relative", flex: 1, minWidth: "200px" }}>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: "0.35rem",
          alignItems: "center",
          background: "rgba(255,255,255,0.05)",
          border: "1px solid var(--color-purple-dim)",
          borderRadius: "var(--radius)",
          padding: "0.35rem 0.5rem",
          cursor: "text",
          minHeight: "2.4rem",
        }}
        onClick={() => inputRef.current?.focus()}
      >
        {value.map((tag) => (
          <span
            key={tag}
            className="badge badge--purple"
            style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
          >
            {tag}
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); removeTag(tag); }}
              style={{
                background: "none",
                border: "none",
                color: "var(--color-purple)",
                cursor: "pointer",
                padding: 0,
                fontSize: "1rem",
                lineHeight: 1,
                opacity: 0.7,
              }}
            >
              x
            </button>
          </span>
        ))}
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => { setInput(e.target.value); setOpen(true); setHighlightIdx(-1); }}
          onFocus={() => setOpen(true)}
          onKeyDown={handleKeyDown}
          placeholder={value.length === 0 ? placeholder : ""}
          style={{
            flex: 1,
            minWidth: "80px",
            background: "transparent",
            border: "none",
            outline: "none",
            color: "var(--color-ink)",
            fontSize: "0.9rem",
            padding: "0.2rem 0",
          }}
        />
      </div>

      {showDropdown && (
        <ul
          style={{
            position: "absolute",
            top: "100%",
            left: 0,
            right: 0,
            marginTop: "4px",
            background: "#0d1033",
            border: "1px solid var(--color-purple-dim)",
            borderRadius: "var(--radius)",
            maxHeight: "200px",
            overflowY: "auto",
            zIndex: 10,
            listStyle: "none",
            padding: "0.25rem 0",
          }}
        >
          {filtered.map((s, i) => (
            <li
              key={s.tag}
              onMouseDown={(e) => { e.preventDefault(); addTag(s.tag); }}
              onMouseEnter={() => setHighlightIdx(i)}
              style={{
                padding: "0.4rem 0.75rem",
                cursor: "pointer",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                fontSize: "0.875rem",
                background: i === highlightIdx ? "var(--color-purple-dim)" : "transparent",
                color: "var(--color-ink)",
              }}
            >
              <span>{s.tag}</span>
              <span style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>{s.count}</span>
            </li>
          ))}
          {input.trim() && !filtered.some((s) => s.tag === input.trim().toLowerCase()) && (
            <li
              onMouseDown={(e) => { e.preventDefault(); addTag(input); }}
              onMouseEnter={() => setHighlightIdx(filtered.length)}
              style={{
                padding: "0.4rem 0.75rem",
                cursor: "pointer",
                fontSize: "0.875rem",
                color: "var(--text-muted)",
                fontStyle: "italic",
                background: highlightIdx === filtered.length ? "var(--color-purple-dim)" : "transparent",
              }}
            >
              Add "{input.trim()}"
            </li>
          )}
        </ul>
      )}
    </div>
  );
}
