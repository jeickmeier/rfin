import fs from "fs";
import path from "path";
import react from "@vitejs/plugin-react";
import dts from "vite-plugin-dts";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => {
  const wasmPkgEntry = path.resolve(
    __dirname,
    "../../finstack-wasm/pkg/finstack_wasm.js",
  );
  const wasmTestStub = path.resolve(__dirname, "src/lib/finstackWasmStub.ts");

  // In CI/test runs we don't rely on built WASM artifacts. The unit tests mock
  // `finstack-wasm`, but Vite still needs a resolvable module during analysis.
  const finstackWasmAlias =
    mode === "test"
      ? wasmTestStub
      : fs.existsSync(wasmPkgEntry)
        ? wasmPkgEntry
        : undefined;

  const alias: Record<string, string> = {
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
  };

  if (finstackWasmAlias) {
    alias["finstack-wasm"] = finstackWasmAlias;
  }

  return {
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
      alias,
      preserveSymlinks: true,
    },
    optimizeDeps: {
      include: ["finstack-wasm"],
    },
    server: {
      fs: {
        allow: [
          // allow loading wasm bundle from the workspace pkg dir
          path.resolve(__dirname),
          path.resolve(__dirname, "../../finstack-wasm/pkg"),
        ],
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
      deps: {
        inline: ["finstack-wasm"],
      },
    },
  };
});
