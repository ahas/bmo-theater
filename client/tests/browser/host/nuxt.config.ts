import { fileURLToPath } from "node:url";

// The theater's screens stood up in a browser with no console around them (`tests/browser/screens.test.ts`): the
// client as a layer, the language packs read up front, the relay answered by the test, and the page drawn in the
// browser alone, since every call it makes is the test's to answer. The build, its output, the bundler's cache and
// what the test draws for the host — its media and the layer's words — stand under the client's own packages, so
// nothing is written beside the sources.

const client = fileURLToPath(new URL("../../..", import.meta.url));
const cache = fileURLToPath(new URL("../../../node_modules/.cache/theater-host", import.meta.url));

export default defineNuxtConfig({
  extends: [client],
  modules: ["@nuxtjs/i18n"],
  ssr: false,
  devtools: { enabled: false },
  telemetry: false,
  buildDir: `${cache}/.nuxt`,
  nitro: { output: { dir: `${cache}/.output` } },
  dir: { public: `${cache}/public` },
  alias: { "#theater-host-words": `${cache}/common.json` },
  vite: {
    cacheDir: `${cache}/vite`,
    // The layer's modules lean on what Nuxt imports for them, so they are never pre-bundled apart from the app.
    optimizeDeps: { include: ["dayjs", "dayjs/plugin/utc", "dayjs/plugin/timezone"], exclude: ["bmo-ui"] }
  },
  i18n: {
    strategy: "no_prefix",
    defaultLocale: "ko",
    locales: ["ko", "en"],
    detectBrowserLanguage: false,
    vueI18n: "./i18n.config.ts"
  },
  icon: { clientBundle: { scan: true } }
});
