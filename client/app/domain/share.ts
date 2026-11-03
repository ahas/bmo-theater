import type { ShareLink, ShareRoad } from "bmo-ui/app/domain/share-links";
import type { Share } from "../../shared/types";

// A share link as the layer's one share dialog reads it: the road its links are read, made and ended by, over
// the service's own (`docs/api.md`, 「Share links」), and the spans a link is offered for.

const DAY_MS = 24 * 60 * 60 * 1000;

/** The spans a link is made for, each its value, the words it is chosen by and how long it lives, none forever. */
export const SHARE_SPANS = [
  { value: "1d", label: "theater.label.span_1d", ms: DAY_MS },
  { value: "7d", label: "theater.label.span_7d", ms: 7 * DAY_MS },
  { value: "30d", label: "theater.label.span_30d", ms: 30 * DAY_MS },
  { value: "forever", label: "common.label.forever", ms: null }
] as const;

/** The span a link is made for first. */
export const FIRST_SPAN = "7d";

/** The address a link opens: the one the service names, else the console's share screen for its token. */
export function sharePage(share: Pick<Share, "token" | "url">): string {
  return share.url ?? `/theater/share/${encodeURIComponent(share.token)}`;
}

interface Links {
  read: () => Promise<Share | null>;
  make: (expiresAt: number | null) => Promise<Share>;
  remove: (share: Pick<Share, "token">) => Promise<void>;
}

/** The dialog's road over the service's links of one member, its clock the moment a span is counted from. */
export function shareRoad(links: Links, now: () => number = Date.now): ShareRoad {
  const pages = new Map<string, string>();

  function known(share: Share): ShareLink {
    pages.set(share.token, sharePage(share));

    return { token: share.token, expiresAt: share.expiresAt === null ? null : Math.floor(share.expiresAt / 1000) };
  }

  return {
    page: (link) => pages.get(link.token) ?? sharePage({ token: link.token, url: null }),
    read: async () => {
      const share = await links.read();

      return share === null ? null : known(share);
    },
    make: async (span) => {
      const ms = SHARE_SPANS.find((one) => one.value === span)?.ms ?? null;

      return known(await links.make(ms === null ? null : now() + ms));
    },
    remove: (link) => links.remove(link)
  };
}
