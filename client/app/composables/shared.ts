import { effectScope } from "vue";
import { useNuxtApp } from "nuxt/app";

// One instance of a composable for the whole app rather than one per caller: the grid's chips, the
// sidebar and the toolbar all read the same calendars, events and draft. Kept on the app itself, so two
// apps rendered side by side never share one.
export function createAppComposable<T>(composable: () => T): () => T {
  const instances = new WeakMap<object, T>();

  return () => {
    const nuxtApp = useNuxtApp();

    if (!instances.has(nuxtApp)) instances.set(nuxtApp, effectScope(true).run(composable)!);

    return instances.get(nuxtApp)!;
  };
}
