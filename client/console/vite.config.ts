import { readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { consoleBundle } from "bmo-ui/bundle/console-bundle.js";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { defineConfig } from "vite";

import type { Plugin } from "vite";

// The served bundle's build, on bmo-ui's bundle build (bmo-ui's `docs/console-bundles.md`, "The layer a bundle is
// built on"): the pages' routes as one browser module, the utility classes they use as one stylesheet, the language
// pack as one words object and the layer the bundle is built on, all written into `dist/`, which the service answers
// under its manifest. The libraries a console shares stay imports of the package's own path, as bmo-ui's plugin
// leaves them. The same sources are typechecked as a Nuxt app over bmo-ui (`nuxt.config.ts`), so what a Nuxt build
// supplies without an import is imported here.

const root = fileURLToPath(new URL("..", import.meta.url));

// The layer's components, by the name a page writes them under.
const LAYER_COMPONENTS = new Set(readdirSync(join(root, "node_modules/bmo-ui/app/components")).map((file) => basename(file, ".vue")));

/** A component of the component library, written `U<Name>`, at its own path in the library. */
function libraryComponent(name: string) {
  return /^U[A-Z]/.test(name) ? { name: "default", from: `@nuxt/ui/components/${name.slice(1)}.vue` } : undefined;
}

/** A component of the layer at its own path in the layer. */
function layerComponent(name: string) {
  return LAYER_COMPONENTS.has(name) ? { name: "default", from: `bmo-ui/app/components/${name}.vue` } : undefined;
}

/** The language pack, one file to a locale, read as one object keyed by the locale's code. */
function readWords(): Record<string, unknown> {
  const folder = join(root, "i18n");
  const locales = readdirSync(folder).filter((file) => file.endsWith(".json")).sort();

  return Object.fromEntries(locales.map((file) => [basename(file, ".json"), JSON.parse(readFileSync(join(folder, file), "utf8"))]));
}

// The bundle's words: what the bundle build holds the pages to, and what the service answers.
const WORDS = readWords();

/** The language pack written as the bundle's words. */
function words(): Plugin {
  return {
    name: "theater-words",
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "words.json", source: JSON.stringify(WORDS) });
    }
  };
}

export default defineConfig({
  root,
  resolve: { alias: { "#build": join(root, ".nuxt") } },
  plugins: [
    AutoImport({
      imports: [
        "vue",
        { "nuxt/app": ["navigateTo", "useAppConfig", "useFetch", "useHead", "useRoute", "useRouter", "useRuntimeConfig"] },
        { [join(root, "console/page-meta.ts")]: ["definePageMeta"] }
      ],
      dts: false
    }),
    Components({ dirs: [join(root, "app/components")], resolvers: [libraryComponent, layerComponent], dts: false }),
    vue(),
    tailwindcss(),
    words(),
    consoleBundle({ name: "theater", words: WORDS })
  ],
  build: {
    outDir: join(root, "dist"),
    emptyOutDir: true,
    lib: { entry: join(root, "console/routes.ts"), formats: ["es"], fileName: () => "bundle.js", cssFileName: "bundle" }
  }
});
