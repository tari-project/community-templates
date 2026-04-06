import type { FunctionDef, TypeDef } from "../api/client";

function formatType(t: TypeDef): string {
  if (typeof t === "string") return t;
  if ("Vec" in t) return `Vec<${formatType(t.Vec)}>`;
  if ("Option" in t) return `Option<${formatType(t.Option)}>`;
  if ("Tuple" in t) return `(${t.Tuple.map(formatType).join(", ")})`;
  if ("Other" in t) return t.Other.name;
  return "?";
}

function isConstructor(f: FunctionDef): boolean {
  // Constructors don't have &self or &mut self as the first argument
  if (f.arguments.length === 0) return true;
  const first = f.arguments[0].name;
  return first !== "self";
}

export default function FunctionSignature({ func }: { func: FunctionDef }) {
  const isCtor = isConstructor(func);

  return (
    <div
      style={{
        padding: "0.75rem 1rem",
        background: "var(--grad-accent)",
        border: "1px solid var(--color-purple-dim)",
        borderRadius: "var(--radius)",
        marginBottom: "0.5rem",
        fontFamily: "var(--font-mono)",
        fontSize: "0.85rem",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
        {isCtor && <span className="badge badge--green">constructor</span>}
        {!isCtor && func.is_mut && <span className="badge badge--purple">mut</span>}
        {!isCtor && !func.is_mut && <span className="badge badge--muted">read</span>}
        <span style={{ color: "var(--color-purple)", fontWeight: 600 }}>
          {func.name}
        </span>
        <span style={{ color: "var(--text-muted)" }}>(</span>
        {func.arguments
          .filter((a) => a.name !== "self")
          .map((arg, i, arr) => (
            <span key={arg.name}>
              <span style={{ color: "var(--color-ink)" }}>{arg.name}</span>
              <span style={{ color: "var(--text-muted)" }}>
                : {formatType(arg.arg_type)}
              </span>
              {i < arr.length - 1 && (
                <span style={{ color: "var(--text-muted)" }}>, </span>
              )}
            </span>
          ))}
        <span style={{ color: "var(--text-muted)" }}>)</span>
        {formatType(func.output) !== "Unit" && (
          <span style={{ color: "var(--text-muted)" }}>
            {" → "}
            <span style={{ color: "var(--color-green)" }}>
              {formatType(func.output)}
            </span>
          </span>
        )}
      </div>
    </div>
  );
}
