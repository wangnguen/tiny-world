import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const page = (file: string) => fileURLToPath(new URL(file, import.meta.url));

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  resolve: {
    dedupe: ["react", "react-dom"],
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "chrome105",
    outDir: "dist",
    // Hai trang: overlay vẽ pet (TS thuần, không kéo React vào) và cửa sổ cài đặt (React).
    rollupOptions: {
      input: {
        overlay: page("index.html"),
        settings: page("settings.html"),
      },
    },
  },
});
