import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const base = process.env.VITE_BASE_PATH ?? "/";
const apiPath = base === "/" ? "/api" : `${base.replace(/\/$/, "")}/api`;

export default defineConfig({
  base,
  plugins: [react()],
  server: {
    proxy: {
      [apiPath]: "http://localhost:3000",
    },
  },
});
