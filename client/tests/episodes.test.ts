import { describe, expect, test } from "bun:test";

import { episodeLabel, episodeName, percent, titleName, watchedWords } from "../app/domain/a11y";
import { episodeCode, episodeTitle, factsOf, followingOf, lengthClock, minutesLeft, placeAfter, placeOf, sectionOf, sectionsOf, watchedEpisode, watchedOf } from "../app/domain/episodes";
import { NEW_PLAYLIST, fileForm, nameOk, playlistChoices, playlistRealms, sentText } from "../app/domain/forms";
import { itemKeys, moved, removedAt } from "../app/domain/order";
import { episode, playlist, title } from "./stand-in";
import { translator } from "./words";

// How the screens read a title's episodes and a playlist's order: an episode's code, its seasons, how far the
// viewer has come, what comes after an episode, an order moved and cut, the playlists an episode may go into, and
// what a reader who does not see the screen is told of each.

const t = translator("en");

describe("an episode's code and seasons", () => {
  test("a show's episode reads S06E35, one with no season E01, one with no numbers nothing", () => {
    expect(episodeCode({ season: 6, number: 35 })).toBe("S06E35");
    expect(episodeCode({ season: null, number: 1 })).toBe("E01");
    expect(episodeCode({ season: null, number: null })).toBe("");
    expect(episodeTitle({ season: 1, number: 2, name: "Trailer" })).toBe("S01E02 Trailer");
  });

  // A show as the service lists it: season 0 first, the seasons, the numberless at the end.
  const show = [
    episode(1, 1, "mine", { season: 0, number: 1 }),
    episode(2, 1, "mine", { season: 0, number: 2 }),
    episode(3, 1, "mine", { season: 1, number: 1 }),
    episode(4, 1, "mine", { season: 1, number: 2 }),
    episode(5, 1, "mine", { season: 2, number: 1 }),
    episode(6, 1, "mine", { season: null, number: null, name: "Trailer" })
  ];

  test("a show's seasons stand in order with the specials last, and its extras apart", () => {
    const sections = sectionsOf("show", show);

    expect(sections.chosen.map((one) => [one.key, one.name(t), one.episodes.map((item) => item.id)])).toEqual([
      ["1", "Season 1", [3, 4]],
      ["2", "Season 2", [5]],
      ["specials", "Specials", [1, 2]]
    ]);
    expect(sections.extras?.episodes.map((item) => item.id)).toEqual([6]);
    expect(sections.extras?.name(t)).toBe("Extras");
    expect(sectionOf(sections, 6)).toBe(sections.extras);
  });

  test("a title that is no show is one whole, and holds nothing apart", () => {
    const sections = sectionsOf("clip", [episode(7, 2, "mine"), episode(8, 2, "mine")]);

    expect(sections.chosen.map((one) => one.episodes.length)).toEqual([2]);
    expect(sections.extras).toBeNull();
  });

  test("what a title is, in one line's figures", () => {
    expect(factsOf(sectionsOf("show", show), show)).toEqual({ seasons: 2, episodes: 6, durationMs: 600_000 });
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
  test("a show goes on along its main line into the next season, specials among themselves, extras nowhere", () => {
    const sections = sectionsOf("show", [
      episode(1, 1, "mine", { season: 0, number: 1 }),
      episode(2, 1, "mine", { season: 0, number: 2 }),
      episode(3, 1, "mine", { season: 1, number: 1 }),
      episode(4, 1, "mine", { season: 1, number: 2 }),
      episode(5, 1, "mine", { season: 2, number: 1 }),
      episode(6, 1, "mine", { season: null, number: null })
    ]);

    expect(followingOf(sections, 3)?.id).toBe(4);
    expect(followingOf(sections, 4)?.id).toBe(5);
    expect(followingOf(sections, 5)).toBeNull();
    expect(followingOf(sections, 1)?.id).toBe(2);
    expect(followingOf(sections, 2)).toBeNull();
    expect(followingOf(sections, 6)).toBeNull();
    expect(followingOf(sections, 9)).toBeNull();
  });

  test("the minutes left of an episode started and not finished, rounded up", () => {
    const started = episode(1, 1, "mine", { durationMs: 600_000, position: { episodeId: 1, seconds: 100, done: false, updatedAt: 1, setAside: false } });

    expect(minutesLeft(started)).toBe(9);
    expect(minutesLeft({ ...started, position: { ...started.position!, done: true } })).toBeNull();
    expect(minutesLeft(episode(2, 1, "mine"))).toBeNull();
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

  // The playlists the reader holds in each realm.
  const held = {
    mine: [playlist(1, "mine", "Weekend", []), playlist(2, "mine", "Harbour cuts", [])],
    organization: [playlist(3, "organization", "Onboarding", [])]
  };

  test("an episode of the reader's own goes into a new playlist or one of theirs, offered with no realm named", () => {
    expect(playlistChoices(t, playlistRealms("mine"), held)).toEqual([
      [{ label: "New playlist", value: NEW_PLAYLIST }],
      [
        { label: "Weekend", value: "1" },
        { label: "Harbour cuts", value: "2" }
      ]
    ]);
  });

  test("an organization episode goes into a playlist of either realm, each realm's group named, an empty realm left out", () => {
    expect(playlistChoices(t, playlistRealms("organization"), held)).toEqual([
      [{ label: "New playlist", value: NEW_PLAYLIST }],
      [
        { type: "label", label: t("theater.label.mine") },
        { label: "Weekend", value: "1" },
        { label: "Harbour cuts", value: "2" }
      ],
      [
        { type: "label", label: t("theater.label.organization") },
        { label: "Onboarding", value: "3" }
      ]
    ]);
    expect(playlistChoices(t, playlistRealms("organization"), { ...held, mine: [] })).toEqual([
      [{ label: "New playlist", value: NEW_PLAYLIST }],
      [
        { type: "label", label: t("theater.label.organization") },
        { label: "Onboarding", value: "3" }
      ]
    ]);
  });
});
