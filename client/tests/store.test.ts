import { describe, expect, setDefaultTimeout, test } from "bun:test";

import { Refusal, createTheaterApi } from "../app/domain/api";
import { readShared, sharedFailure } from "../app/domain/shared";
import { createTheaterStore, refusalWord } from "../app/domain/store";
import { episode, playlist, startStandIn, title, unreachable } from "./stand-in";

import type { Notice } from "../app/domain/store";
import type { StandIn } from "./stand-in";

// The screens' hold on the theater service, against a stand-in of its routes: the realms kept apart, a title's
// progress and what to watch next, a playlist's order sent whole and taken back when refused, a video and a
// subtitle filed as multipart bodies, a burn followed by its events, whether the reader administers the
// theater, what a share link opens, and an episode's grants as the share dialog's member road reads them.

// A test's bound fits a loaded machine, where a few loopback calls can take seconds. Each test holds its own
// stand-in and takes it down as its body ends, so one that runs past its bound keeps its server until it is done,
// and no test's end takes away the server another test is still calling.
setDefaultTimeout(60_000);

function storeOver(stand: StandIn) {
  const notices: Notice[] = [];
  const store = createTheaterStore({ api: createTheaterApi(stand.call, stand.send), tell: (notice) => notices.push(notice) });

  return { store, notices };
}

describe("continue watching", () => {
  test("is read in the order the service answers, and an episode set aside leaves it at once", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine"), episode(12, 1, "mine"), episode(13, 1, "mine")] });

    const { store } = storeOver(standIn);
    const api = createTheaterApi(standIn.call, standIn.send);

    await api.setPosition(11, 10);
    await api.setPosition(12, 20);
    await api.setPosition(13, 95);
    await store.loadWatching("mine");

    expect(store.watching.value.mine?.map((one) => one.id)).toEqual([12, 11]);

    expect(await store.setAside(store.watching.value.mine![0]!)).toBe(true);
    expect(store.watching.value.mine?.map((one) => one.id)).toEqual([11]);
    expect(standIn.calls).toContain("DELETE /watching/12");

    await store.loadWatching("mine");
    expect(store.watching.value.mine?.map((one) => one.id)).toEqual([11]);
  });

  test("a position set anywhere counts as a change the rows read again", () => {
    using standIn = startStandIn();

    const { store } = storeOver(standIn);
    const before = store.rowsChanged.value;

    store.apply({ kind: "position.changed", data: { episodeId: 11, seconds: 40, done: false, updatedAt: 99, setAside: false } });

    expect(store.rowsChanged.value).toBe(before + 1);
  });
});

describe("the realms", () => {
  test("each realm's titles are read and kept apart", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot"), title(2, "organization", "Trailer")] });

    const { store } = storeOver(standIn);

    await Promise.all([store.loadTitles("mine"), store.loadTitles("organization")]);

    expect(store.titles.value.mine.map((one) => one.name)).toEqual(["Pilot"]);
    expect(store.titles.value.organization.map((one) => one.name)).toEqual(["Trailer"]);
    expect(standIn.calls).toContain("GET /titles?realm=mine&order=added");
  });

  test("an event puts a title in its realm, moves it with its realm and takes it out when deleted", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")] });

    const { store } = storeOver(standIn);

    await store.loadTitles("mine");
    store.apply({ kind: "title.created", data: title(5, "organization", "Trailer") });
    store.apply({ kind: "title.changed", data: title(1, "organization", "Pilot") });

    expect(store.titles.value.mine).toEqual([]);
    // Held in the service's order of the one added to last first, a tie by the newer id.
    expect(store.titles.value.organization.map((one) => one.id)).toEqual([5, 1]);

    store.apply({ kind: "title.deleted", data: { id: 5 } });

    expect(store.titles.value.organization.map((one) => one.id)).toEqual([1]);
  });
});

describe("progress and the next episode", () => {
  test("a title's progress names the next episode, and a position set moves it on", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine"), episode(12, 1, "mine"), episode(13, 1, "mine")] });

    const { store } = storeOver(standIn);

    await store.loadEpisodes(1);
    expect((await store.loadProgress("title", 1))?.next).toBe(11);

    const position = await store.setPosition(11, 95);

    expect(position).toMatchObject({ episodeId: 11, done: true });
    expect(store.episodeOf(11)?.position?.done).toBe(true);
    expect(store.progressOf("title", 1)?.items[0]).toMatchObject({ episodeId: 11, seconds: 95, done: true });
    expect((await store.loadProgress("title", 1))?.next).toBe(12);
  });

  test("an episode stopped short is the one to go on from, at the second it stopped", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine"), episode(12, 1, "mine")] });

    const { store } = storeOver(standIn);

    await store.setPosition(12, 30);

    expect(await store.loadProgress("title", 1)).toMatchObject({ next: 12, nextSeconds: 30, doneCount: 0 });
  });

  test("a position the stream tells of is put on the episode and the progress holding it", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { store } = storeOver(standIn);

    await store.loadEpisodes(1);
    await store.loadProgress("title", 1);
    store.apply({ kind: "position.changed", data: { episodeId: 11, seconds: 40, done: false, updatedAt: 99 } });

    expect(store.episodeOf(11)?.position?.seconds).toBe(40);
    expect(store.progressOf("title", 1)?.items[0]?.seconds).toBe(40);
  });

  test("an episode read by its id never stands for its title's whole list", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine"), episode(12, 1, "mine")] });

    const { store } = storeOver(standIn);

    await store.loadEpisodesByIds([12]);

    expect(store.episodeOf(12)?.id).toBe(12);
    expect(store.episodes.value[1]).toBeUndefined();
  });
});

describe("episodes read by their ids", () => {
  test("two asks for one episode are one call", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { store } = storeOver(standIn);

    await Promise.all([store.loadEpisodesByIds([11]), store.loadEpisodesByIds([11, 11])]);

    expect(standIn.calls.filter((call) => call === "GET /episodes/11")).toEqual(["GET /episodes/11"]);
  });
});

describe("a playlist's order", () => {
  function seeded() {
    return startStandIn({
      titles: [title(1, "organization", "Pilot")],
      episodes: [episode(11, 1, "organization"), episode(12, 1, "organization"), episode(13, 1, "organization")],
      playlists: [playlist(7, "organization", "Season", [11, 12, 13, 11])]
    });
  }

  test("a move sends the whole order in place of the one standing", async () => {
    using standIn = seeded();

    const { store } = storeOver(standIn);
    const read = await store.loadPlaylist(7);

    expect(await store.reorderPlaylist(read, 2, 0)).toBe(true);
    expect(standIn.bodies.get("PATCH /playlists/7")).toEqual({ episodeIds: [13, 11, 12, 11] });
    expect(store.playlistOf(7)?.episodeIds).toEqual([13, 11, 12, 11]);
  });

  test("removing an item takes out that place alone, an episode named twice keeping its other place", async () => {
    using standIn = seeded();

    const { store } = storeOver(standIn);
    const read = await store.loadPlaylist(7);

    await store.removeFromPlaylist(read, 3);

    expect(standIn.playlists[0]?.episodeIds).toEqual([11, 12, 13]);
  });

  test("a refused move puts the order back and says why", async () => {
    using standIn = seeded();
    standIn.refused.add("playlist:7");

    const { store, notices } = storeOver(standIn);
    const read = await store.loadPlaylist(7);

    expect(await store.reorderPlaylist(read, 0, 1)).toBe(false);
    expect(store.playlistOf(7)?.episodeIds).toEqual([11, 12, 13, 11]);
    expect(notices).toEqual([{ tone: "error", title: "theater.error.forbidden" }]);
  });

  test("a playlist's progress answers an episode named twice twice", async () => {
    using standIn = seeded();

    const { store } = storeOver(standIn);

    expect((await store.loadProgress("playlist", 7))?.items.map((item) => item.episodeId)).toEqual([11, 12, 13, 11]);
  });
});

describe("refused writes", () => {
  test("a reader's own episode is refused in an organization playlist, in the words of its code", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { store } = storeOver(standIn);
    const refused = await store.makePlaylist("organization", "Shared", [11]).catch((error: unknown) => error);

    expect(refused).toBeInstanceOf(Refusal);
    expect((refused as Refusal).code).toBe("not-in-realm");
    expect(refusalWord(refused)).toBe("theater.error.not_in_realm");
  });

  test("a library's episode is not deleted, and the refusal is told", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { store, notices } = storeOver(standIn);
    const [held] = await store.loadEpisodes(1);

    expect(await store.deleteEpisode(held!)).toBe(false);
    expect(notices[0]?.title).toBe("theater.error.conflict");
    expect(store.episodeOf(11)).toBeDefined();
  });

  test("a call that never landed is told apart from a refusal", async () => {
    const api = createTheaterApi(unreachable, () => Promise.reject(new Error("down")));
    const failed = await api.titles("mine").catch((error: unknown) => error);

    expect(refusalWord(failed)).toBe("theater.error.unreachable");
  });

  test("a subtitle of an unknown format is refused in the words of its code", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { store } = storeOver(standIn);
    const form = new FormData();

    form.append("file", new File(["x"], "lines.txt"));

    expect(refusalWord(await store.addSubtitle(11, form).catch((error: unknown) => error))).toBe("theater.error.unknown_format");
  });
});

describe("filing and subtitles", () => {
  test("a video filed under a new name makes a clip title and sends the file with its name", async () => {
    using standIn = startStandIn();

    const { store } = storeOver(standIn);
    const form = new FormData();
    const progress: number[] = [];

    form.append("file", new File(["bytes"], "intro.mp4"));
    form.append("name", "Introduction");

    const filed = await store.file({ realm: "mine", name: "Outputs" }, form, (sent, total) => progress.push(sent / total));
    const made = store.titles.value.mine[0];

    expect(made).toMatchObject({ name: "Outputs", kind: "clip" });
    expect(filed).toMatchObject({ titleId: made?.id, kind: "clip", name: "Introduction" });
    expect(standIn.forms.get(`/titles/${made?.id}/episodes`)).toEqual({ file: "file:intro.mp4", name: "Introduction" });
    expect(progress.at(-1)).toBe(1);
  });

  test("a subtitle is added and deleted, and a burn is followed by its events", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { store } = storeOver(standIn);
    const form = new FormData();

    form.append("file", new File(["1\n00:00:01,000 --> 00:00:02,000\nHello\n"], "lines.srt"));
    form.append("language", "ko");

    const added = await store.addSubtitle(11, form);

    expect(store.subtitles.value[11]?.map((one) => one.language)).toEqual(["ko"]);

    const burn = await store.burn(11, added.id);

    expect(store.burnOf(11, added.id)?.state).toBe("running");

    store.apply({ kind: "burn.finished", data: { ...burn!, state: "finished", clipId: 99, endedAt: 50 } });

    expect(store.burnOf(11, added.id)).toMatchObject({ state: "finished", clipId: 99 });
    expect(await store.deleteSubtitle(added)).toBe(true);
    expect(store.subtitles.value[11]).toEqual([]);
  });
});

describe("the administrator", () => {
  test("a member is told apart from the administrator, and the asking registers nothing", async () => {
    using member = startStandIn({ role: "member" });
    using administrator = startStandIn({ role: "administrator" });

    expect(await createTheaterApi(member.call, member.send).administers()).toBe(false);
    expect(await createTheaterApi(administrator.call, administrator.send).administers()).toBe(true);
    expect(administrator.libraries).toEqual([]);
  });

  test("a member's registering of a folder is refused", async () => {
    using standIn = startStandIn({ role: "member" });

    const { store } = storeOver(standIn);

    expect(refusalWord(await store.makeLibrary("mine", "/media", null).catch((error: unknown) => error))).toBe("theater.error.forbidden");
  });
});

describe("the share page", () => {
  test("a playlist link opens its playlist and its episodes in its order, with no position", async () => {
    using standIn = startStandIn({
      titles: [title(1, "organization", "Pilot")],
      episodes: [episode(11, 1, "organization", { position: { episodeId: 11, seconds: 5, done: false, updatedAt: 1, setAside: false } }), episode(12, 1, "organization")],
      playlists: [playlist(7, "organization", "Season", [12, 11])],
      shares: [{ token: "tokenA", episodeId: null, playlistId: 7, url: null, creator: "2", createdAt: 1, expiresAt: null }]
    });

    const api = createTheaterApi(standIn.call, standIn.send);
    const opened = await readShared(api.shared, "tokenA");

    expect(opened.playlist?.name).toBe("Season");
    expect(opened.episodes.map((one) => [one.id, one.position])).toEqual([
      [12, null],
      [11, null]
    ]);
    expect(standIn.calls.every((call) => call.includes("/shares/tokenA"))).toBe(true);
  });

  test("an episode link opens that episode alone and no playlist", async () => {
    using standIn = startStandIn({
      titles: [title(1, "mine", "Pilot")],
      episodes: [episode(11, 1, "mine"), episode(12, 1, "mine")],
      shares: [{ token: "tokenB", episodeId: 12, playlistId: null, url: null, creator: "2", createdAt: 1, expiresAt: null }]
    });

    const opened = await readShared(createTheaterApi(standIn.call, standIn.send).shared, "tokenB");

    expect(opened.playlist).toBeNull();
    expect(opened.episodes.map((one) => one.id)).toEqual([12]);
  });

  test("an ended link and a missing one are told apart", async () => {
    using standIn = startStandIn({ shares: [{ token: "ended", episodeId: 1, playlistId: null, url: null, creator: "2", createdAt: 1, expiresAt: 1 }] });

    const shared = createTheaterApi(standIn.call, standIn.send).shared;

    expect(sharedFailure(await readShared(shared, "ended").catch((error: unknown) => error))).toBe("theater.error.share_gone");
    expect(sharedFailure(await readShared(shared, "nothing").catch((error: unknown) => error))).toBe("theater.error.share_missing");
  });

  test("a link made for a span ends that long after it is made, read in the dialog's seconds", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { shareRoad } = await import("../app/domain/share");
    const { store } = storeOver(standIn);
    const road = shareRoad(store.shareRoad({ episodeId: 11 }), () => 1_000_000);
    const made = await road.make("1d");

    expect(standIn.bodies.get("POST /shares")).toEqual({ episodeId: 11, expiresAt: 1_000_000 + 86_400_000 });
    expect(made.expiresAt).toBe(Math.floor((1_000_000 + 86_400_000) / 1000));
    expect(road.page(made)).toBe(`/theater/share/${made.token}`);
    expect(await road.read()).toEqual(made);
  });
});

describe("sharing with people", () => {
  const people = {
    search: async (words: string) => (words === "ki" ? [{ user: "5", name: "Kim", organization: "Studio", jobTitle: "Editor" }] : []),
    person: async (user: string) => (user === "5" ? { user, name: "Kim", organization: "Studio", jobTitle: "Editor" } : null)
  };

  test("an episode's grants are kept by the service and drawn by the names the directory gives", async () => {
    using standIn = startStandIn({ titles: [title(1, "mine", "Pilot")], episodes: [episode(11, 1, "mine")] });

    const { memberRoad } = await import("../app/domain/share");
    const { store } = storeOver(standIn);
    const road = memberRoad(people, store.grantRoad(11));

    expect((await road.search("ki")).map((person) => person.name)).toEqual(["Kim"]);
    await road.share(["5", "a b"], "read");
    expect(standIn.bodies.get("POST /episodes/11/grants")).toEqual({ users: ["5", "a b"], permission: "read" });
    expect(await road.read()).toEqual([
      { person: { user: "5", name: "Kim", organization: "Studio", jobTitle: "Editor" }, permission: "read" },
      { person: { user: "a b", name: "a b", organization: "", jobTitle: "" }, permission: "read" }
    ]);

    await road.change("a b", "edit");
    expect(standIn.calls).toContain("PUT /episodes/11/grants/a%20b");
    await road.remove("5");
    expect((await road.read()).map((member) => [member.person.user, member.permission])).toEqual([["a b", "edit"]]);
  });
});

