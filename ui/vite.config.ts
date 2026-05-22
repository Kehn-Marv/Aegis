import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// During `npm run dev`, Vite serves the UI at http://localhost:5173 and
// proxies /api/* and /mcp/* through to the daemon at 127.0.0.1:7321 so
// the browser sees same-origin requests.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:7321",
      "/mcp": "http://127.0.0.1:7321",
    },
  },
});
