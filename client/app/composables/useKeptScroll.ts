import { nextTick, onBeforeUnmount, onMounted, watch } from "vue";
import { useRoute } from "nuxt/app";

import type { Ref } from "vue";

// A list found as it was left when what it led to is closed: its offset written down under the address as it
// is made, and put back once the list stands again.

const offsets = new Map<string, number>();

export function useKeptScroll(scroller: Readonly<Ref<HTMLElement | null>>, ready: () => boolean): void {
  const route = useRoute();
  const key = route.fullPath;
  let restored = false;

  function note(): void {
    if (restored && scroller.value) offsets.set(key, scroller.value.scrollTop);
  }

  async function restore(): Promise<void> {
    if (restored || !ready() || !scroller.value) return;

    restored = true;
    await nextTick();
    if (scroller.value) scroller.value.scrollTop = offsets.get(key) ?? 0;
  }

  watch(ready, () => void restore());

  onMounted(() => {
    scroller.value?.addEventListener("scroll", note, { passive: true });
    void restore();
  });

  onBeforeUnmount(() => scroller.value?.removeEventListener("scroll", note));
}
