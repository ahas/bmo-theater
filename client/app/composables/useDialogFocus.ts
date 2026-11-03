import { nextTick, watch } from "vue";

import type { Ref } from "vue";

// Where the focus goes as a dialog opened from a press comes and goes. Coming, it goes to the dialog's first
// field, handed over by the screen, so Escape closes the dialog at the first press rather than a tooltip on its
// close button; a finger's screen is left alone, so no keyboard rises over it. Going, it goes back to what held
// it, or, for a dialog chosen from a menu, to the button that opened the menu, since the menu is gone. Where the
// focus was nowhere in particular, it is left where it lands.

// How many frames a dialog is given to settle, opening or closing, before the focus is moved over wherever the
// dialog library set it.
const SETTLE_FRAMES = 2;

const FIRST_FIELD = 'input:not([type="file"]):not([disabled]), textarea:not([disabled]), button[role="combobox"]:not([disabled])';

async function settled(): Promise<void> {
  await nextTick();

  for (let frame = 0; frame < SETTLE_FRAMES; frame++) await new Promise((resolve) => requestAnimationFrame(resolve));
}

export function useDialogFocus(open: Ref<boolean>): void {
  let held: HTMLElement | null = null;

  watch(open, async (opened) => {
    if (opened) {
      const focused = document.activeElement?.closest('[role="menu"]') ? document.querySelector('[aria-haspopup="menu"][aria-expanded="true"]') : document.activeElement;

      held = focused instanceof HTMLElement && focused !== document.body ? focused : null;
      await settled();

      if (!window.matchMedia("(pointer: coarse)").matches) document.querySelector<HTMLElement>(`[role="dialog"][data-state="open"] :is(${FIRST_FIELD})`)?.focus({ preventScroll: true });

      return;
    }

    const back = held;

    held = null;

    if (back === null) return;

    await settled();

    if (back.isConnected && !document.querySelector('[role="dialog"][data-state="open"]')) back.focus({ preventScroll: true });
  });
}
