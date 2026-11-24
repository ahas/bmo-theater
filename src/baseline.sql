-- The theater's schema at 1.0.0, whole (docs/design.md, 「The schema」).

CREATE TABLE public.burns (
    id bigint NOT NULL,
    episode_id bigint NOT NULL,
    subtitle_id bigint NOT NULL,
    state text NOT NULL,
    clip_id bigint,
    error text,
    creator text NOT NULL,
    started_at bigint NOT NULL,
    ended_at bigint
);

CREATE SEQUENCE public.burns_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.burns_id_seq OWNED BY public.burns.id;

CREATE TABLE public.episodes (
    id bigint NOT NULL,
    title_id bigint NOT NULL,
    kind text NOT NULL,
    season integer,
    number integer,
    name text NOT NULL,
    label text DEFAULT ''::text NOT NULL,
    path text NOT NULL,
    library_id bigint,
    library_key text,
    file_name text NOT NULL,
    media_type text NOT NULL,
    byte_count bigint NOT NULL,
    modified_at bigint NOT NULL,
    duration_ms bigint,
    width integer,
    height integer,
    video_codec text,
    audio_codec text,
    available boolean DEFAULT true NOT NULL,
    has_poster boolean DEFAULT false NOT NULL,
    burned_episode bigint,
    burned_subtitle bigint,
    creator text NOT NULL,
    created_at bigint NOT NULL,
    changed_at bigint NOT NULL,
    poster_width integer,
    poster_height integer
);

CREATE SEQUENCE public.episodes_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.episodes_id_seq OWNED BY public.episodes.id;

CREATE TABLE public.grants (
    episode_id bigint NOT NULL,
    account text NOT NULL,
    permission text NOT NULL,
    grantor text NOT NULL,
    granted_at bigint NOT NULL,
    CONSTRAINT grants_permission_check CHECK ((permission = ANY (ARRAY['read'::text, 'edit'::text])))
);

CREATE TABLE public.libraries (
    id bigint NOT NULL,
    realm text NOT NULL,
    owner text NOT NULL,
    creator text NOT NULL,
    name text NOT NULL,
    path text NOT NULL,
    created_at bigint NOT NULL,
    scanned_at bigint
);

CREATE SEQUENCE public.libraries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.libraries_id_seq OWNED BY public.libraries.id;

CREATE TABLE public.playlist_items (
    playlist_id bigint NOT NULL,
    ord integer NOT NULL,
    episode_id bigint NOT NULL
);

CREATE TABLE public.playlists (
    id bigint NOT NULL,
    realm text NOT NULL,
    owner text NOT NULL,
    creator text NOT NULL,
    name text NOT NULL,
    created_at bigint NOT NULL,
    changed_at bigint NOT NULL
);

CREATE SEQUENCE public.playlists_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.playlists_id_seq OWNED BY public.playlists.id;

CREATE TABLE public.positions (
    account text NOT NULL,
    episode_id bigint NOT NULL,
    seconds double precision NOT NULL,
    done boolean NOT NULL,
    updated_at bigint NOT NULL,
    set_aside boolean DEFAULT false NOT NULL
);

CREATE TABLE public.scans (
    id bigint NOT NULL,
    library_id bigint NOT NULL,
    state text NOT NULL,
    found_count bigint DEFAULT 0 NOT NULL,
    added_count bigint DEFAULT 0 NOT NULL,
    changed_count bigint DEFAULT 0 NOT NULL,
    missing_count bigint DEFAULT 0 NOT NULL,
    error text,
    started_at bigint NOT NULL,
    ended_at bigint
);

CREATE SEQUENCE public.scans_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.scans_id_seq OWNED BY public.scans.id;

CREATE TABLE public.shares (
    token text NOT NULL,
    episode_id bigint,
    playlist_id bigint,
    creator text NOT NULL,
    created_at bigint NOT NULL,
    expires_at bigint,
    CONSTRAINT shares_check CHECK (((episode_id IS NULL) <> (playlist_id IS NULL)))
);

CREATE TABLE public.subtitles (
    id bigint NOT NULL,
    episode_id bigint NOT NULL,
    language text NOT NULL,
    label text DEFAULT ''::text NOT NULL,
    format text NOT NULL,
    file_name text NOT NULL,
    cue_count bigint NOT NULL,
    creator text NOT NULL,
    created_at bigint NOT NULL
);

CREATE SEQUENCE public.subtitles_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.subtitles_id_seq OWNED BY public.subtitles.id;

CREATE TABLE public.titles (
    id bigint NOT NULL,
    realm text NOT NULL,
    owner text NOT NULL,
    creator text NOT NULL,
    name text NOT NULL,
    kind text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    labels text DEFAULT '{}'::text NOT NULL,
    library_id bigint,
    library_key text,
    retired boolean DEFAULT false NOT NULL,
    created_at bigint NOT NULL,
    changed_at bigint NOT NULL
);

CREATE SEQUENCE public.titles_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.titles_id_seq OWNED BY public.titles.id;

ALTER TABLE ONLY public.burns ALTER COLUMN id SET DEFAULT nextval('public.burns_id_seq'::regclass);

ALTER TABLE ONLY public.episodes ALTER COLUMN id SET DEFAULT nextval('public.episodes_id_seq'::regclass);

ALTER TABLE ONLY public.libraries ALTER COLUMN id SET DEFAULT nextval('public.libraries_id_seq'::regclass);

ALTER TABLE ONLY public.playlists ALTER COLUMN id SET DEFAULT nextval('public.playlists_id_seq'::regclass);

ALTER TABLE ONLY public.scans ALTER COLUMN id SET DEFAULT nextval('public.scans_id_seq'::regclass);

ALTER TABLE ONLY public.subtitles ALTER COLUMN id SET DEFAULT nextval('public.subtitles_id_seq'::regclass);

ALTER TABLE ONLY public.titles ALTER COLUMN id SET DEFAULT nextval('public.titles_id_seq'::regclass);

ALTER TABLE ONLY public.burns
    ADD CONSTRAINT burns_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.episodes
    ADD CONSTRAINT episodes_library_id_library_key_key UNIQUE (library_id, library_key);

ALTER TABLE ONLY public.episodes
    ADD CONSTRAINT episodes_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.grants
    ADD CONSTRAINT grants_pkey PRIMARY KEY (episode_id, account);

ALTER TABLE ONLY public.libraries
    ADD CONSTRAINT libraries_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.libraries
    ADD CONSTRAINT libraries_realm_owner_path_key UNIQUE (realm, owner, path);

ALTER TABLE ONLY public.playlist_items
    ADD CONSTRAINT playlist_items_pkey PRIMARY KEY (playlist_id, ord);

ALTER TABLE ONLY public.playlists
    ADD CONSTRAINT playlists_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.positions
    ADD CONSTRAINT positions_pkey PRIMARY KEY (account, episode_id);

ALTER TABLE ONLY public.scans
    ADD CONSTRAINT scans_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_pkey PRIMARY KEY (token);

ALTER TABLE ONLY public.subtitles
    ADD CONSTRAINT subtitles_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.titles
    ADD CONSTRAINT titles_library_id_library_key_key UNIQUE (library_id, library_key);

ALTER TABLE ONLY public.titles
    ADD CONSTRAINT titles_pkey PRIMARY KEY (id);

CREATE INDEX burns_episode ON public.burns USING btree (episode_id, id);

CREATE INDEX episodes_title ON public.episodes USING btree (title_id);

CREATE INDEX playlist_items_episode ON public.playlist_items USING btree (episode_id);

CREATE INDEX scans_library ON public.scans USING btree (library_id, id);

CREATE UNIQUE INDEX scans_running ON public.scans USING btree (library_id) WHERE (state = 'running'::text);

CREATE UNIQUE INDEX shares_episode ON public.shares USING btree (episode_id) WHERE (episode_id IS NOT NULL);

CREATE UNIQUE INDEX shares_playlist ON public.shares USING btree (playlist_id) WHERE (playlist_id IS NOT NULL);

CREATE INDEX subtitles_episode ON public.subtitles USING btree (episode_id);

ALTER TABLE ONLY public.burns
    ADD CONSTRAINT burns_episode_id_fkey FOREIGN KEY (episode_id) REFERENCES public.episodes(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.episodes
    ADD CONSTRAINT episodes_library_id_fkey FOREIGN KEY (library_id) REFERENCES public.libraries(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.episodes
    ADD CONSTRAINT episodes_title_id_fkey FOREIGN KEY (title_id) REFERENCES public.titles(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.grants
    ADD CONSTRAINT grants_episode_id_fkey FOREIGN KEY (episode_id) REFERENCES public.episodes(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.playlist_items
    ADD CONSTRAINT playlist_items_episode_id_fkey FOREIGN KEY (episode_id) REFERENCES public.episodes(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.playlist_items
    ADD CONSTRAINT playlist_items_playlist_id_fkey FOREIGN KEY (playlist_id) REFERENCES public.playlists(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.positions
    ADD CONSTRAINT positions_episode_id_fkey FOREIGN KEY (episode_id) REFERENCES public.episodes(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.scans
    ADD CONSTRAINT scans_library_id_fkey FOREIGN KEY (library_id) REFERENCES public.libraries(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_episode_id_fkey FOREIGN KEY (episode_id) REFERENCES public.episodes(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_playlist_id_fkey FOREIGN KEY (playlist_id) REFERENCES public.playlists(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.subtitles
    ADD CONSTRAINT subtitles_episode_id_fkey FOREIGN KEY (episode_id) REFERENCES public.episodes(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.titles
    ADD CONSTRAINT titles_library_id_fkey FOREIGN KEY (library_id) REFERENCES public.libraries(id) ON DELETE CASCADE;

