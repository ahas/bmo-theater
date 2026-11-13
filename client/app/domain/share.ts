import type { ShareLink, ShareRoad } from "bmo-ui/app/domain/share-links";
import type { ShareMemberRoad, SharePerson } from "bmo-ui/app/domain/share-members";
import type { Grant, Permission, Share } from "../../shared/types";

// What the layer's one share dialog reads of a shared thing: the road its links are read, made and ended by,
// over the service's own (`docs/api.md`, 「Share links」), with the spans a link is offered for; and the road
// its members go by, people found through the console's directory and kept as the service's grants
// (`docs/api.md`, 「Grants」).

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

/** The key the console offers its people directory under (bmo-ui `docs/console-bundles.md`, the people directory). */
export const CONSOLE_PEOPLE = "bmo-console-people";

/** What the screens read of the console's people directory: the reader's company searched, one account by its key. */
export interface ConsolePeople {
  search: (words: string) => Promise<SharePerson[]>;
  person: (user: string) => Promise<SharePerson | null>;
}

interface Grants {
  read: () => Promise<Grant[]>;
  grant: (users: string[], permission: Permission) => Promise<void>;
  change: (user: string, permission: Permission) => Promise<void>;
  remove: (user: string) => Promise<void>;
}

/** The dialog's road over one episode's grants: each drawn by the name the directory gives its account, else by
 * the key alone. */
export function memberRoad(people: ConsolePeople, grants: Grants): ShareMemberRoad {
  async function personOf(user: string): Promise<SharePerson> {
    const found = await people.person(user).catch(() => null);

    return found ?? { user, name: user, organization: "", jobTitle: "" };
  }

  return {
    search: (words) => people.search(words),
    read: async () => Promise.all((await grants.read()).map(async (grant) => ({ person: await personOf(grant.user), permission: grant.permission }))),
    share: grants.grant,
    change: grants.change,
    remove: grants.remove
  };
}
