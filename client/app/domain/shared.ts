import { Refusal, Unreachable } from "./api";

import type { SharedReads } from "./api";
import type { Episode, Playlist, Share } from "../../shared/types";

// What a share link opens, read with its token alone (`docs/api.md`, 「Share links」): the link, the playlist
// where it opens one, and its episodes in the playlist's order.

export interface SharedOpening {
  link: Share;
  playlist: Playlist | null;
  episodes: Episode[];
}

export async function readShared(reads: SharedReads, token: string): Promise<SharedOpening> {
  const link = await reads.link(token);
  const [playlist, episodes] = await Promise.all([link.playlistId === null ? Promise.resolve(null) : reads.playlist(token), reads.episodes(token)]);

  return { link, playlist, episodes };
}

/** The words a share page says it cannot open its link in: ended, not standing, or not reached. */
export function sharedFailure(error: unknown): string {
  if (error instanceof Unreachable) return "theater.error.unreachable";
  if (error instanceof Refusal && error.status === 410) return "theater.error.share_gone";
  if (error instanceof Refusal && error.status === 404) return "theater.error.share_missing";

  return "theater.error.share_failed";
}
