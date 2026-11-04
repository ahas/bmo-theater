import { describe, expect, test } from "bun:test";

import { DONE_SHARE, firstTrack, keyAction, positionMoved, primaryLanguage, skipTo, startAt, steppedVolume, toggledTrack, trackName , nextTurn, otherOrientation, turnedBox } from "../app/domain/player";

import type { Subtitle } from "../shared/types";

// How the player behaves apart from the element it drives: its keys, its skips, where it starts, when it sets
// a position again, and which subtitle it shows first.

function subtitle(id: number, language: string, label = ""): Pick<Subtitle, "id" | "language" | "label" | "fileName"> {
  return { id, language, label, fileName: `lines-${id}.srt` };
}

describe("the keys", () => {
  test("space plays and pauses, the arrows skip and move the volume, f, m and c do their own", () => {
    expect([" ", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "f", "m", "c"].map(keyAction)).toEqual(["toggle", "back", "forward", "louder", "quieter", "fullscreen", "mute", "captions"]);
  });

  test("r turns the picture", () => {
    expect(keyAction("r")).toBe("turn");
  });

  test("answer no other key", () => {
    expect(keyAction("Enter")).toBeNull();
    expect(keyAction("x")).toBeNull();
  });
});

describe("skips and volume", () => {
  test("a skip lands inside the video", () => {
    expect(skipTo(5, -10, 100)).toBe(0);
    expect(skipTo(95, 10, 100)).toBe(100);
    expect(skipTo(50, 10, null)).toBe(60);
  });

  test("a volume step stays between nought and one", () => {
    expect(steppedVolume(0.95, 0.1)).toBe(1);
    expect(steppedVolume(0.05, -0.1)).toBe(0);
    expect(steppedVolume(0.5, 0.1)).toBe(0.6);
  });
});

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

  test("the captions key turns the shown one off, and back to the last one shown", () => {
    expect(toggledTrack(2, null, tracks)).toBeNull();
    expect(toggledTrack(null, 2, tracks)).toBe(2);
    expect(toggledTrack(null, null, tracks)).toBe(1);
    expect(toggledTrack(null, 9, tracks)).toBe(1);
    expect(toggledTrack(null, null, [])).toBeNull();
  });

  test("is named by its label, else its language, else its file", () => {
    expect(trackName(subtitle(1, "en", "Director"), "en")).toBe("Director");
    expect(trackName(subtitle(1, "ko"), "en")).toBe("Korean");
    expect(trackName(subtitle(1, "und"), "en")).toBe("lines-1.srt");
  });
});

describe("turning the picture", () => {
  test("goes a quarter at a time and comes round", () => {
    expect([0, 90, 180, 270].map(nextTurn)).toEqual([90, 180, 270, 0]);
  });

  test("asks for the other orientation than the screen stands in", () => {
    expect(otherOrientation("portrait-primary")).toBe("landscape");
    expect(otherOrientation("landscape-primary")).toBe("portrait");
    expect(otherOrientation(undefined)).toBe("landscape");
  });

  test("fits a picture on its side to the player, with its width the player's height", () => {
    expect(turnedBox(0, 400, 800)).toEqual({});
    expect(turnedBox(90, 400, 800)).toMatchObject({ width: "800px", height: "400px", transform: "translate(-50%, -50%) rotate(90deg)" });
    expect(turnedBox(180, 400, 800)).toMatchObject({ width: "400px", height: "800px" });
  });
});
