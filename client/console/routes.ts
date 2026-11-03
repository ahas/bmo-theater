import LibraryPage from "../app/pages/theater/index.vue";
import LibrariesPage from "../app/pages/theater/libraries.vue";
import PlaylistPage from "../app/pages/theater/playlists/[id].vue";
import PlaylistsPage from "../app/pages/theater/playlists/index.vue";
import SharePage from "../app/pages/theater/share/[token].vue";
import TitlePage from "../app/pages/theater/titles/[id].vue";
import WatchPage from "../app/pages/theater/watch/[id].vue";

import "./screens.css";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";

// The console bundle's module (bmo-ui's `docs/console-bundles.md`): the theater's screens under the screen path,
// each under the console's title bar, and the share page standing alone with no frame of the console's.
const bundle: ConsoleBundleModule = {
  routes: [
    { path: "/theater", name: "theater", component: LibraryPage, meta: { consoleTitleBar: true } },
    { path: "/theater/titles/:id", name: "theater-titles-id", component: TitlePage, meta: { consoleTitleBar: true } },
    { path: "/theater/watch/:id", name: "theater-watch-id", component: WatchPage, meta: { consoleTitleBar: true } },
    { path: "/theater/playlists", name: "theater-playlists", component: PlaylistsPage, meta: { consoleTitleBar: true } },
    { path: "/theater/playlists/:id", name: "theater-playlists-id", component: PlaylistPage, meta: { consoleTitleBar: true } },
    { path: "/theater/libraries", name: "theater-libraries", component: LibrariesPage, meta: { consoleTitleBar: true } },
    { path: "/theater/share/:token", name: "theater-share-token", component: SharePage, meta: { layout: false } }
  ]
};

export default bundle;
