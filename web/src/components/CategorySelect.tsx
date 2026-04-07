import { useEffect, useRef, useState } from "react";
import type { CategoryCount } from "../api/client";

interface CategorySelectProps {
  value: string;
  onChange: (category: string) => void;
  suggestions: CategoryCount[];
  placeholder?: string;
}

export default function CategorySelect({ value, onChange, suggestions, placeholder = "Category..." }: CategorySelectProps) {
  const [input, setInput] = useState(value);
  const [open, setOpen] = useState(false);
  const [highlightIdx, setHighlightIdx] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sync external value changes
  useEffect(() => { setInput(value); }, [value]);

  const filtered = suggestions.filter(
    (s) => s.category.toLowerCase().includes(input.toLowerCase()),
  );

  const select = (cat: string) => {
    setInput(cat);
    onChange(cat);
    setOpen(false);
    setHighlightIdx(-1);
  };

  const clear = () => {
    setInput("");
    onChange("");
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (highlightIdx >= 0 && highlightIdx < filtered.length) {
        select(filtered[highlightIdx].category);
      } else if (input.trim()) {
        select(input.trim());
      }
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

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div ref={containerRef} style={{ position: "relative", flex: 1, minWidth: "120px" }}>
      <div style={{ position: "relative" }}>
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => { setInput(e.target.value); onChange(e.target.value); setOpen(true); setHighlightIdx(-1); }}
          onFocus={() => setOpen(true)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          style={{ width: "100%", paddingRight: value ? "2rem" : undefined }}
        />
        {value && (
          <button
            type="button"
            onClick={clear}
            style={{
              position: "absolute",
              right: "0.5rem",
              top: "50%",
              transform: "translateY(-50%)",
              background: "none",
              border: "none",
              color: "var(--text-muted)",
              cursor: "pointer",
              padding: 0,
              fontSize: "1rem",
              lineHeight: 1,
            }}
          >
            x
          </button>
        )}
      </div>

      {open && filtered.length > 0 && (
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
              key={s.category}
              onMouseDown={(e) => { e.preventDefault(); select(s.category); }}
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
              <span>{s.category}</span>
              <span style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>{s.count}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
