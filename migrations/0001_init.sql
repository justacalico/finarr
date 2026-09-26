-- Finarr initial schema.

CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user')),
    disabled INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_login_at TEXT
);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    token_hash TEXT NOT NULL UNIQUE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'session' CHECK (kind IN ('session', 'api_key')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at TEXT,
    last_seen_at TEXT
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE indexers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    api_key TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 25,
    categories TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE download_clients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    impl TEXT NOT NULL CHECK (impl IN ('builtin', 'qbittorrent')),
    settings TEXT NOT NULL DEFAULT '{}',
    priority INTEGER NOT NULL DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE movies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tmdb_id INTEGER UNIQUE,
    imdb_id TEXT,
    title TEXT NOT NULL,
    sort_title TEXT NOT NULL DEFAULT '',
    original_title TEXT,
    year INTEGER,
    overview TEXT NOT NULL DEFAULT '',
    poster_url TEXT,
    backdrop_url TEXT,
    runtime_min INTEGER,
    genres TEXT NOT NULL DEFAULT '[]',
    monitored INTEGER NOT NULL DEFAULT 1,
    path TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'missing' CHECK (status IN ('missing', 'downloading', 'imported')),
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    release_date TEXT,
    digital_date TEXT,
    min_availability TEXT NOT NULL DEFAULT 'released' CHECK (min_availability IN ('announced', 'in_cinemas', 'released'))
);

CREATE TABLE series (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tvmaze_id INTEGER,
    tvdb_id INTEGER,
    tmdb_id INTEGER,
    imdb_id TEXT,
    title TEXT NOT NULL,
    sort_title TEXT NOT NULL DEFAULT '',
    overview TEXT NOT NULL DEFAULT '',
    poster_url TEXT,
    backdrop_url TEXT,
    year INTEGER,
    network TEXT,
    air_time TEXT,
    series_status TEXT NOT NULL DEFAULT 'unknown',
    monitored INTEGER NOT NULL DEFAULT 1,
    path TEXT NOT NULL DEFAULT '',
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(tvmaze_id)
);

CREATE TABLE seasons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    season_number INTEGER NOT NULL,
    monitored INTEGER NOT NULL DEFAULT 1,
    UNIQUE(series_id, season_number)
);

CREATE TABLE episodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    season_number INTEGER NOT NULL,
    episode_number INTEGER NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    overview TEXT NOT NULL DEFAULT '',
    air_date TEXT,
    runtime_min INTEGER,
    monitored INTEGER NOT NULL DEFAULT 1,
    file_id INTEGER,
    status TEXT NOT NULL DEFAULT 'missing' CHECK (status IN ('missing', 'downloading', 'imported', 'unaired')),
    UNIQUE(series_id, season_number, episode_number)
);

CREATE TABLE artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mbid TEXT UNIQUE,
    name TEXT NOT NULL,
    sort_name TEXT NOT NULL DEFAULT '',
    overview TEXT NOT NULL DEFAULT '',
    image_url TEXT,
    monitored INTEGER NOT NULL DEFAULT 1,
    path TEXT NOT NULL DEFAULT '',
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    mbid TEXT,
    title TEXT NOT NULL,
    release_date TEXT,
    album_type TEXT NOT NULL DEFAULT 'album',
    monitored INTEGER NOT NULL DEFAULT 1,
    file_id INTEGER,
    status TEXT NOT NULL DEFAULT 'missing' CHECK (status IN ('missing', 'downloading', 'imported')),
    UNIQUE(artist_id, mbid)
);

CREATE TABLE media_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    quality TEXT NOT NULL DEFAULT '',
    movie_id INTEGER REFERENCES movies(id) ON DELETE CASCADE,
    episode_id INTEGER REFERENCES episodes(id) ON DELETE CASCADE,
    album_id INTEGER REFERENCES albums(id) ON DELETE CASCADE,
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE download_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT,
    client_item_id TEXT,
    client TEXT NOT NULL DEFAULT 'builtin',
    name TEXT NOT NULL,
    media_type TEXT CHECK (media_type IN ('movie', 'series', 'album', 'manual') OR media_type IS NULL),
    movie_id INTEGER REFERENCES movies(id) ON DELETE SET NULL,
    episode_ids TEXT NOT NULL DEFAULT '[]',
    album_id INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    release_title TEXT NOT NULL DEFAULT '',
    indexer TEXT NOT NULL DEFAULT '',
    save_path TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    size_bytes INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'queued',
    progress REAL NOT NULL DEFAULT 0,
    imported INTEGER NOT NULL DEFAULT 0,
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    completed_at TEXT
);

CREATE TABLE requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_type TEXT NOT NULL CHECK (media_type IN ('movie', 'series', 'artist')),
    external_id TEXT NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    poster_url TEXT,
    detail TEXT NOT NULL DEFAULT '{}',
    requested_by INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'declined', 'fulfilled')),
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    resolved_at TEXT
);

CREATE TABLE history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    media_type TEXT NOT NULL DEFAULT '',
    ref_id INTEGER,
    title TEXT NOT NULL DEFAULT '',
    data TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_episodes_series ON episodes(series_id);
CREATE INDEX idx_episodes_wanted ON episodes(monitored, status, air_date);
CREATE INDEX idx_movies_wanted ON movies(monitored, status);
CREATE INDEX idx_albums_artist ON albums(artist_id);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_history_created ON history(created_at);
CREATE INDEX idx_download_items_hash ON download_items(hash);
CREATE INDEX idx_requests_status ON requests(status);
