import { resolve } from "node:path";
import { defineConfig } from "vite";

// Two pages: the editor (index.html) and the region-selection overlay
// (overlay.html). Tauri opens each by path, so both must be Rollup inputs.
export default defineConfig({
  // Tauri owns the console output; Vite clearing it hides Rust panics.
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome110",
    rollupOptions: {
      input: {
        editor: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
      },
    },
  },
});
