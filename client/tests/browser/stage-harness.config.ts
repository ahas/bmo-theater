import { fileURLToPath } from "node:url";

import ui from "@nuxt/ui/vite";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

// The player stage's harness page built into one script (`tests/browser/player-stage.test.ts`): Vue, the component
// library, the theater's stage and the layer's player it draws, so a page with no console around it draws them.
// The test hands it an output folder.

const here = (path: string): string => fileURLToPath(new URL(path, import.meta.url));
const layer = (path: string): string => here(`../../node_modules/bmo-ui/${path}`);

// The layer's app configuration is written for Nuxt, which supplies the helper it is wrapped in.
(globalThis as { defineAppConfig?: (config: unknown) => unknown }).defineAppConfig = (config) => config;

const { default: appConfig } = (await import("bmo-ui/app/app.config.ts")) as { default: { ui: Record<string, unknown> & { icons: Record<string, string> } } };

const { icons } = appConfig.ui;

export default defineConfig({
  root: here("../.."),
  // The script runs in the browser alone: what Nuxt states of the side a file runs on is stated here.
  define: { "import.meta.client": "true", "import.meta.server": "false", "import.meta.dev": "false", "process.env.NODE_ENV": JSON.stringify("production") },
  plugins: [
    vue(),
    ui({
      router: false,
      colorMode: false,
      dts: false,
      ui: appConfig.ui,
      // The layer's files lean on Vue's names as Nuxt hands them over, so the package is read for them too.
      autoImport: { imports: ["vue"], exclude: [/[\\/]node_modules[\\/](?!bmo-ui[\\/])/, /[\\/]\.git[\\/]/], dts: false },
      // The layer's player is compiled here from the package, so the parts it names are found in the package too.
      scanPackages: ["bmo-ui"],
      components: { dirs: [layer("app/components")], dts: false },
      // The icons travel in the script, since the page fetches nothing but it.
      icon: {
        clientBundle: {
          icons: [icons.loading!, icons.close!, icons.check!],
          scan: { globInclude: ["node_modules/bmo-ui/app/components/VideoPlayer.vue", "app/components/theater/PlayerStage.vue"] }
        }
      }
    })
  ],
  build: {
    emptyOutDir: true,
    lib: { entry: here("./stage-harness.ts"), formats: ["iife"], name: "StageHarness", fileName: () => "stage-harness.js" }
  }
});
