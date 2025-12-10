import path from "path";
import react from "@vitejs/plugin-react";
import dts from "vite-plugin-dts";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    react(),
    dts({
      entryRoot: "src",
      tsconfigPath: "./tsconfig.build.json",
      outDir: "dist",
      include: ["src/**/*"],
      insertTypesEntry: true,
    }),
  ],
  resolve: {
    alias: {
      "@finstack-ui/components": path.resolve(__dirname, "src/components"),
      "@finstack-ui/hooks": path.resolve(__dirname, "src/hooks"),
      "@finstack-ui/utils": path.resolve(__dirname, "src/utils"),
      "@finstack-ui/workers": path.resolve(__dirname, "src/workers"),
      "@finstack-ui/types": path.resolve(__dirname, "src/types"),
      "@finstack-ui/lib": path.resolve(__dirname, "src/lib"),
      "@finstack-ui/store": path.resolve(__dirname, "src/store"),
      "@/components": path.resolve(__dirname, "src/components"),
      "@/hooks": path.resolve(__dirname, "src/hooks"),
      "@/utils": path.resolve(__dirname, "src/utils"),
      "@/workers": path.resolve(__dirname, "src/workers"),
      "@/types": path.resolve(__dirname, "src/types"),
      "@/lib": path.resolve(__dirname, "src/lib"),
      "@/store": path.resolve(__dirname, "src/store"),
    },
  },
  worker: {
    format: "es",
  },
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      name: "FinstackUI",
      fileName: "index",
      formats: ["es"],
    },
    sourcemap: true,
    rollupOptions: {
      external: [
        "react",
        "react-dom",
        "finstack-wasm",
        "comlink",
        "zustand",
        "zod",
      ],
      output: {
        globals: {
          react: "React",
          "react-dom": "ReactDOM",
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./vitest.setup.ts",
    globals: true,
  },
});
