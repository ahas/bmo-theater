import { describe, expect, test } from "bun:test";

import { DONE_SHARE, firstTrack, positionMoved, primaryLanguage, startAt, trackName } from "../app/domain/player";

import type { Subtitle } from "../shared/types";

// What the theater decides around the layer's player: where an episode starts, when it sets a position again,
// and which subtitle it shows first and by what name.

function subtitle(id: number, language: string, label = ""): Pick<Subtitle, "id" | "language" | "label" | "fileName"> {
  return { id, language, label, fileName: `lines-${id}.srt` };
}

describe("where an episode starts", () => {
  test("where the viewer stopped, unless it is watched or that is past nine tenths of it", () => {
    expect(startAt(null, 100)).toBe(0);
    expect(startAt({ seconds: 40, done: false }, 100)).toBe(40);
    expect(startAt({ seconds: 40, done: true }, 100)).toBe(0);
    expect(startAt({ seconds: 100 * DONE_SHARE, done: false }, 100)).toBe(0);
    expect(startAt({ seconds: 40, done: false }, null)).toBe(40);
  });

  test("a position is set again only once it has moved a second or more", () => {
    expect(positionMoved(null, 0)).toBe(true);
    expect(positionMoved(10, 10.5)).toBe(false);
    expect(positionMoved(10, 11)).toBe(true);
  });
});

describe("the subtitle shown first", () => {
  const tracks = [subtitle(1, "en"), subtitle(2, "ko-KR"), subtitle(3, "und")];

  test("is the one in the screen's language, by its primary subtag", () => {
    expect(primaryLanguage("ko-KR")).toBe("ko");
    expect(firstTrack(tracks, "ko")).toBe(2);
    expect(firstTrack(tracks, "en-US")).toBe(1);
  });

  test("is none where none is in the screen's language, so subtitles start off", () => {
    expect(firstTrack(tracks, "ja")).toBeNull();
    expect(firstTrack([], "ko")).toBeNull();
    expect(firstTrack([subtitle(3, "und")], "und")).toBeNull();
  });

  test("is named by its label, else its language, else its file", () => {
    expect(trackName(subtitle(1, "en", "Director"), "en")).toBe("Director");
    expect(trackName(subtitle(1, "ko"), "en")).toBe("Korean");
    expect(trackName(subtitle(1, "und"), "en")).toBe("lines-1.srt");
  });
});
