import { followSurface, useLongPress } from "bmo-ui/app/domain/longpress";

import { attachGestures } from "../domain/gestures";

import type { GestureReader, GestureSpec } from "../domain/gestures";

// A finger on the player's picture (`docs/design.md`, 「The console」). The reader is the one state machine of
// `domain/gestures`; the half-second that makes a press a hold is the layer's one long press
// (`docs/components.md`, "A held finger"), so a slide before it, a scroll and a second finger end it the same way
// they end every other hold of the interface.

export interface PlayerGestureSpec extends GestureSpec {
  /** The picture the finger lands on, followed as the player opens another stream. */
  surface: () => HTMLVideoElement | null;
}

export function usePlayerGestures(spec: PlayerGestureSpec): void {
  let reader: GestureReader | null = null;

  followSurface(
    spec.surface,
    (picture) => {
      reader = attachGestures(picture as HTMLVideoElement, spec);
    },
    () => {
      reader?.dispose();
      reader = null;
    }
  );

  useLongPress({
    surface: spec.surface,
    press: (target) => {
      const picture = spec.surface();

      if (picture === null || spec.blocked() || !(target instanceof Element) || !picture.contains(target)) return null;
      if (target.closest("button, a, input, textarea, select, [contenteditable]") !== null) return null;

      return () => reader?.hold();
    }
  });
}
