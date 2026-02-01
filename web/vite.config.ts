import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import { resolvePort } from "./scripts/ports";

export default defineConfig(() => {
  const webDevPort = resolvePort("webDev").port;
  const webPreviewPort = resolvePort("webPreview").port;

  return {
    base: "/",
    plugins: [react(), tailwindcss()],
    server: {
      port: webDevPort,
      strictPort: true,
    },
    preview: {
      port: webPreviewPort,
      strictPort: true,
    },
  };
});
