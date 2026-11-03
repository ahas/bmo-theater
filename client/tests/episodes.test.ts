import { describe, expect, test } from "bun:test";

import { episodeLabel, episodeName, percent, titleName, watchedWords } from "../app/domain/a11y";
import { episodeAfter, episodeCode, episodeTitle, lengthClock, placeAfter, placeOf, seasonsOf, watchedEpisode, watchedOf } from "../app/domain/episodes";
import { fileForm, nameOk, sentText } from "../app/domain/forms";
import { itemKeys, moved, removedAt } from "../app/domain/order";
import { episode, title } from "./stand-in";
import { translator } from "./words";

// How the screens read a title's episodes and a playlist's order: an episode's code, its seasons, how far the
// viewer has come, what comes after an episode, an order moved and cut, and what a reader who does not see
// the screen is told of each.

const t = translator("en");

describe("an episode's code and seasons", () => {
  test("a show's episode reads S06E35, one with no season E01, one with no numbers nothing", () => {
    expect(episodeCode({ season: 6, number: 35 })).toBe("S06E35");
    expect(episodeCode({ season: null, number: 1 })).toBe("E01");
    expect(episodeCode({ season: null, number: null })).toBe("");
    expect(episodeTitle({ season: 1, number: 2, name: "Trailer" })).toBe("S01E02 Trailer");
  });

  test("seasons stand in order, the episodes with none last", () => {
    const seasons = seasonsOf([episode(1, 1, "mine", { season: 2 }), episode(2, 1, "mine", { season: null }), episode(3, 1, "mine", { season: 1 }), episode(4, 1, "mine", { season: 2 })]);

    expect(seasons.map((one) => [one.season, one.episodes.map((item) => item.id)])).toEqual([
      [1, [3]],
      [2, [1, 4]],
      [null, [2]]
    ]);
  });
});

describe("how far the viewer has come", () => {
  test("a share of the length, whole once watched, nothing where nothing is known", () => {
    expect(watchedOf({ seconds: 25, done: false, durationMs: 100_000 }).fraction).toBe(0.25);
    expect(watchedOf({ seconds: 10, done: true, durationMs: 100_000 }).fraction).toBe(1);
    expect(watchedOf({ seconds: 10, done: false, durationMs: null }).fraction).toBe(0);
    expect(watchedOf(undefined).fraction).toBe(0);
    expect(watchedEpisode(episode(1, 1, "mine")).fraction).toBe(0);
  });

  test("said in words to a reader who does not see the bar", () => {
    expect(watchedWords(t, watchedOf({ seconds: 25, done: false, durationMs: 100_000 }))).toBe("25% watched");
    expect(watchedWords(t, watchedOf({ seconds: 1, done: true, durationMs: 100_000 }))).toBe("Watched");
    expect(percent(0.999)).toBe(99);
  });

  test("an episode row and a title card are named whole", () => {
    expect(episodeName(t, episode(3, 1, "mine", { season: 1, number: 3, name: "Pilot", label: "burned: ko" }), watchedOf(null))).toBe("S01E03 Pilot, 1:40, burned: ko, Not watched");
    expect(titleName(t, title(1, "mine", "Pilot", { kind: "film", episodeCount: 1 }), (n) => n.toLocaleString("en-US"))).toBe("Pilot, Film, Episodes 1");
  });

  test("a burned clip's label is read in the screen's words, any other as it stands", () => {
    const burned = episode(3, 1, "mine", { label: "burned: ko", burnedFrom: { episodeId: 1, subtitleId: 2 } });

    expect(episodeLabel(t, burned)).toBe("Burned in: ko");
    expect(episodeLabel(t, episode(4, 1, "mine", { label: "Director" }))).toBe("Director");
  });

  test("a length reads as a clock", () => {
    expect(lengthClock(100_000)).toBe("1:40");
    expect(lengthClock(3_725_000)).toBe("1:02:05");
    expect(lengthClock(null)).toBe("");
  });
});

describe("what comes next", () => {
  test("a title's next episode is the one after in its order, none at its end", () => {
    const list = [{ id: 1 }, { id: 2 }, { id: 3 }];

    expect(episodeAfter(list, 1)).toBe(2);
    expect(episodeAfter(list, 3)).toBeNull();
    expect(episodeAfter(list, 9)).toBeNull();
  });

  test("a playlist's next is the place after, an episode named twice opened at the place asked", () => {
    expect(placeAfter([5, 6, 5], 0)).toBe(1);
    expect(placeAfter([5, 6, 5], 2)).toBeNull();
    expect(placeOf([5, 6, 5], 5, 2)).toBe(2);
    expect(placeOf([5, 6, 5], 5, 1)).toBe(0);
    expect(placeOf([5, 6, 5], 7, null)).toBeNull();
  });
});

describe("a playlist's order", () => {
  test("an item moved takes its new place and the rest close up", () => {
    expect(moved([1, 2, 3, 4], 0, 2)).toEqual([2, 3, 1, 4]);
    expect(moved([1, 2, 3, 4], 3, 0)).toEqual([4, 1, 2, 3]);
    expect(moved([1, 2], 0, 5)).toEqual([1, 2]);
  });

  test("an item removed by its place leaves its repeat standing", () => {
    expect(removedAt([1, 2, 1], 0)).toEqual([2, 1]);
  });

  test("each item keeps a key of its own, a repeat told apart from the first", () => {
    expect(itemKeys([1, 2, 1])).toEqual(["1:0", "2:0", "1:1"]);
  });
});

describe("the forms", () => {
  test("a file is sent with the text parts given and none left blank", () => {
    const form = fileForm(new File(["x"], "intro.mp4"), { name: " Intro ", label: "", season: null });

    expect((form.get("file") as File).name).toBe("intro.mp4");
    expect(form.get("name")).toBe("Intro");
    expect(form.has("label")).toBe(false);
    expect(form.has("season")).toBe(false);
  });

  test("a name is on one line and not blank, and a sending reads as its megabytes and its share", () => {
    expect(nameOk("Season")).toBe(true);
    expect(nameOk("  ")).toBe(false);
    expect(nameOk("a\nb")).toBe(false);
    expect(sentText(12_300_000, 450_000_000, "en-US")).toBe("12.3/450.0 MB 2%");
  });
});
