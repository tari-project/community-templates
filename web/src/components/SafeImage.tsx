import { useState } from "react";

const ALLOWED_EXTENSIONS = [".png", ".jpg", ".jpeg", ".svg", ".webp", ".gif"];

function isValidLogoUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:") return false;
    const path = parsed.pathname.toLowerCase();
    return ALLOWED_EXTENSIONS.some((ext) => path.endsWith(ext));
  } catch {
    return false;
  }
}

export default function SafeImage({
  url,
  alt,
  size,
}: {
  url: string | null;
  alt: string;
  size: number;
}) {
  const [errored, setErrored] = useState(false);

  if (!url || !isValidLogoUrl(url) || errored) {
    return (
      <div
        style={{
          width: size,
          height: size,
          borderRadius: "var(--radius)",
          background: "var(--color-purple-dim)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: size * 0.4,
          color: "var(--color-purple)",
          fontFamily: "var(--font-heading)",
          flexShrink: 0,
        }}
      >
        {alt.charAt(0).toUpperCase()}
      </div>
    );
  }

  return (
    <img
      src={url}
      alt={alt}
      referrerPolicy="no-referrer"
      crossOrigin="anonymous"
      onError={() => setErrored(true)}
      style={{
        width: size,
        height: size,
        borderRadius: "var(--radius)",
        objectFit: "cover",
        flexShrink: 0,
      }}
    />
  );
}
