import { computed } from "vue";
import { navigateTo, useRoute } from "nuxt/app";

import type { Realm } from "../../shared/types";

// The realm a screen shows, kept in its address (`?realm=organization`), so a link opens the same tab: the
// reader's own where none is named.

export function realmOf(asked: unknown): Realm {
  return asked === "organization" ? "organization" : "mine";
}

export function useRealm() {
  const route = useRoute();
  const realm = computed<Realm>({
    get: () => realmOf(route.query.realm),
    set: (value) => void navigateTo({ path: route.path, query: { ...route.query, realm: value === "mine" ? undefined : value } })
  });

  return { realm };
}
