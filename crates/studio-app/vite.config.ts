import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1422,
    strictPort: true,
  },
  test: {
    // The colour-token test asserts on the stylesheet Tailwind actually ships,
    // so CSS has to go through the compiler instead of being stubbed out.
    css: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "esnext",
    outDir: "dist",
    rollupOptions: {
      external: ["@tauri-apps/api"],
    },
  },
});