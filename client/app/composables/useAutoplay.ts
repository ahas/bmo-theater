import { ref } from "vue";

// Whether the next episode follows on its own (`docs/design.md`, 「The console」, what plays next): the viewer's
// own switch, kept in the browser that set it, since the service keeps no settings of a viewer. On where never set;
// where the browser keeps nothing, the switch holds for the page alone.

const KEY = "bmo-theater-autoplay";

function readKept(): boolean {
  try {
    return globalThis.localStorage?.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

const autoplay = ref(readKept());

export function useAutoplay() {
  function setAutoplay(on: boolean): void {
    autoplay.value = on;

    try {
      globalThis.localStorage?.setItem(KEY, on ? "on" : "off");
    } catch {
      // A browser that keeps nothing keeps the switch for this page alone.
    }
  }

  return { autoplay, setAutoplay };
}
