import type { FunctionDef, TypeDef } from "../api/client";

function formatType(t: TypeDef): string {
  if (typeof t === "string") return t;
  if ("Vec" in t) return `Vec<${formatType(t.Vec)}>`;
  if ("Option" in t) return `Option<${formatType(t.Option)}>`;
  if ("Tuple" in t) return `(${t.Tuple.map(formatType).join(", ")})`;
  if ("Other" in t) return t.Other.name;
  return "?";
}

function isMethod(f: FunctionDef): boolean {
  return f.arguments.length > 0 && f.arguments[0].name === "self";
}

function isConstructor(f: FunctionDef): boolean {
  // A non-method is only a constructor if it returns Self or Component<...>.
  // Anything else is a plain associated function.
  if (isMethod(f)) return false;
  const out = f.output;
  if (typeof out !== "object" || !("Other" in out)) return false;
  const name = out.Other.name;
  return name === "Self" || name.startsWith("Component<");
}

export default function FunctionSignature({ func }: { func: FunctionDef }) {
  const isCtor = isConstructor(func);
  const isMeth = isMethod(func);

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
        {isMeth && func.is_mut && <span className="badge badge--purple">mut</span>}
        {isMeth && !func.is_mut && <span className="badge badge--muted">read</span>}
        {!isCtor && !isMeth && <span className="badge badge--muted">fn</span>}
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
