import { useI18n } from "vue-i18n";
import { relayAddress, relayCall } from "bmo-ui/app/domain/relay";
import { useIsland } from "bmo-ui/app/domain/island";

import { createTheaterApi } from "../domain/api";
import { createTheaterStore } from "../domain/store";
import { browserSend } from "../domain/upload";
import { createAppComposable } from "./shared";

import type { RelayOptions } from "bmo-ui/app/domain/relay";
import type { Call } from "../domain/api";

// What the screens hold of the theater, read and written through the console's relay to the theater service,
// with the words the store has for the reader put on the console's message pill.

export const SERVICE = "theater";

/** The address a path of the service is relayed at: a picture's, a stream's, a subtitle's source. */
export function theaterAddress(path: string): string {
  return relayAddress(SERVICE, path);
}

const relay: Call = <T>(path: string, options?: Parameters<Call>[1]) => relayCall<T>(SERVICE, path, options as RelayOptions);

/** The api through the relay, for a screen that holds nothing of the store, as a share page. */
export function theaterApi() {
  return createTheaterApi(relay, browserSend(theaterAddress));
}

const _useTheater = () => {
  const { t } = useI18n();
  const island = useIsland(t);
  const store = createTheaterStore({
    api: theaterApi(),
    tell: (notice) => island.say({ tone: notice.tone, title: t(notice.title, notice.params ?? {}) })
  });

  return { ...store, island };
};

export const useTheater = createAppComposable(_useTheater);
