// The theater screens as a Nuxt app over bmo-ui, for the typecheck alone: what a console runs is the bundle
// `console/vite.config.ts` builds, which the service answers under its manifest (docs/api.md, 「Console」).
export default defineNuxtConfig({
  extends: ["bmo-ui"],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false }
});
