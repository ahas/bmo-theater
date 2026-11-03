import { useTheaterStream } from "../domain/stream";
import { SERVICE, useTheater } from "./useTheater";

import type { StreamEvent } from "../../shared/types";

// The service's event stream kept open while a screen stands: each event put where the store holds what it
// tells of, and the screen's own `onEvent` told of it after; a stream opened afresh, after the service stopped
// keeping what was missed, has the screen read what it shows again.

export function useTheaterEvents(onReset: () => void, onEvent: (event: StreamEvent) => void = () => {}): void {
  const { apply } = useTheater();

  useTheaterStream(
    SERVICE,
    (event) => {
      apply(event);
      onEvent(event);
    },
    onReset
  );
}
