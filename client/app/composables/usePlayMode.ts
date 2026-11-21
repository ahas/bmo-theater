import { ref } from "vue";

// How the player's screen is laid out on a wide screen (`docs/design.md`, 「The console」, the play modes): normal,
// the player beside the list, or theater, the player across the whole width. The viewer's own choice, kept in the
// browser that made it, since the service keeps no settings of a viewer; normal where never set, and for the page
// alone where the browser keeps nothing. Full screen is the player's own and is not kept.

/** A play mode the screen keeps. */
export type PlayMode = "normal" | "theater";

const KEY = "bmo-theater-play-mode";

function readKept(): PlayMode {
  try {
    return globalThis.localStorage?.getItem(KEY) === "theater" ? "theater" : "normal";
  } catch {
    return "normal";
  }
}

const mode = ref<PlayMode>(readKept());

export function usePlayMode() {
  function setMode(next: PlayMode): void {
    mode.value = next;

    try {
      globalThis.localStorage?.setItem(KEY, next);
    } catch {
      // A browser that keeps nothing keeps the mode for this page alone.
    }
  }

  return { mode, setMode };
}
