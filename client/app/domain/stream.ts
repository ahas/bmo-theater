import { onBeforeUnmount, onMounted } from "vue";
import { relayAddress } from "bmo-ui/app/domain/relay";

import { STREAM_KINDS } from "../../shared/types";

import type { StreamEvent, StreamKind } from "../../shared/types";

// The theater service's event stream as the screens read it (`docs/api.md`, 「Events the theater publishes」),
// through the console's relay. The browser resumes it after the last event it read; where the service no
// longer keeps the events since then it refuses the stream, and the screen opens it afresh and reads what it
// shows again.

// How long a stream the service refused waits before it is opened afresh.
const REOPEN_MS = 3_000;

/** Calls `onEvent` with each event while the screen holding it stands, and `onReset` each time it is opened afresh. */
export function useTheaterStream(service: string, onEvent: (event: StreamEvent) => void, onReset: () => void): void {
  let source: EventSource | null = null;
  let reopening: ReturnType<typeof setTimeout> | null = null;
  let standing = false;

  function read(message: MessageEvent<string>): void {
    try {
      const envelope = JSON.parse(message.data) as { kind?: string; data?: unknown };

      onEvent({ kind: (envelope.kind ?? message.type) as StreamKind, data: envelope.data ?? envelope });
    } catch {
      // An event that does not read is passed over; the next one says where things stand.
    }
  }

  function open(): void {
    source = new EventSource(relayAddress(service, `/events?kinds=${STREAM_KINDS.join(",")}`));

    for (const kind of STREAM_KINDS) source.addEventListener(kind, read);

    source.onerror = () => {
      if (source?.readyState !== EventSource.CLOSED || !standing) return;

      source = null;
      reopening = setTimeout(() => {
        reopening = null;

        if (!standing) return;

        open();
        onReset();
      }, REOPEN_MS);
    };
  }

  onMounted(() => {
    standing = true;
    open();
  });

  onBeforeUnmount(() => {
    standing = false;
    source?.close();
    source = null;

    if (reopening !== null) clearTimeout(reopening);
  });
}
