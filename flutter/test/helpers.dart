import 'dart:convert';

import 'package:finarr/api.dart';
import 'package:finarr/app_state.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Canned API responses keyed by request path.
dynamic fakeApi(String path, Uri uri) {
  return switch (path) {
    '/api/status' => {
        'version': '1.0.0',
        'setup_required': false,
        'instance_name': 'Finarr',
        'dev_mode': false
      },
    '/api/auth/me' => {
        'id': 1,
        'username': 'admin',
        'display_name': 'Admin',
        'role': 'admin'
      },
    '/api/torrents' => {
        'speeds': {'download': 102400, 'upload': 51200},
        'torrents': [fakeTorrent]
      },
    '/api/torrents/abc123/files' => {
        'files': [
          {
            'index': 0,
            'path': 'Movie.2024.1080p/movie.mkv',
            'size': 4000000000,
            'downloaded': 2100000000,
            'included': true
          },
          {
            'index': 1,
            'path': 'Movie.2024.1080p/sample.mkv',
            'size': 50000000,
            'downloaded': 0,
            'included': false
          },
        ]
      },
    '/api/torrents/abc123' => fakeTorrent,
    '/api/movies' => {'movies': [fakeMovie]},
    '/api/movies/1' => {'movie': fakeMovie, 'files': []},
    '/api/series' => {'series': [fakeSeries]},
    '/api/series/1' => {'series': fakeSeries},
    '/api/series/1/episodes' => {
        'seasons': [
          {'id': 10, 'season_number': 1, 'monitored': 1}
        ],
        'episodes': [fakeEpisode]
      },
    '/api/artists' => {'artists': [fakeArtist]},
    '/api/artists/1' => {
        'artist': fakeArtist,
        'albums': [fakeAlbum]
      },
    '/api/requests' => {'requests': [fakeRequest]},
    '/api/activity/queue' => {'items': [fakeQueueItem]},
    '/api/activity/history' => {'history': [fakeHistoryEvent]},
    '/api/indexers' => {'indexers': [fakeIndexer]},
    '/api/clients' => {'clients': []},
    '/api/users' => {
        'users': [
          {'id': 1, 'username': 'admin', 'role': 'admin'}
        ]
      },
    '/api/calendar' => {'items': [fakeCalendarItem]},
    '/api/wanted' => {
        'movies': [fakeWantedMovie],
        'episodes': [],
        'albums': []
      },
    '/api/settings' => fakeSettings,
    '/api/system/info' => {
        'version': '1.0.0',
        'sqlite_version': '3.46',
        'data_dir': '/data',
        'listen_addr': '0.0.0.0:8787',
        'uptime_seconds': 3600
      },
    '/api/system/disk' => [
        {'path': '/data', 'free_bytes': 500000000000, 'total_bytes': 1000000000000}
      ],
    '/api/system/logs' => {
        'entries': [
          {
            'ts': '2025-01-01T12:00:00Z',
            'level': 'INFO',
            'target': 'finarr',
            'message': 'test log line'
          }
        ]
      },
    _ => <String, dynamic>{},
  };
}

final fakeTorrent = {
  'id': 'abc123',
  'hash': 'abc123',
  'name': 'Some.Movie.2024.1080p.WEB-DL.x264-GROUP',
  'state': 'live',
  'finished': false,
  'progress': 0.52,
  'progress_bytes': 2100000000,
  'total_bytes': 4000000000,
  'uploaded_bytes': 500000000,
  'download_speed': 102400,
  'upload_speed': 51200,
  'eta_seconds': 18000,
  'peers': {'live': 12, 'seen': 45},
  'save_path': '/data/downloads/Some.Movie.2024.1080p.WEB-DL.x264-GROUP',
  'category': '',
  'error': '',
};

final fakeMovie = {
  'id': 1,
  'title': 'Big Movie',
  'original_title': 'Big Movie',
  'year': 2024,
  'overview': 'A test movie about testing.',
  'runtime_min': 112,
  'genres': '["Drama","Thriller"]',
  'tmdb_id': 123,
  'imdb_id': 'tt1234567',
  'monitored': 1,
  'status': 'imported',
  'path': '/data/library/movies/Big Movie (2024)',
  'poster_url': '',
  'backdrop_url': '',
  'created_at': '2025-01-01T00:00:00',
};

final fakeSeries = {
  'id': 1,
  'title': 'Great Show',
  'original_title': '',
  'year': 2023,
  'overview': 'A show that never ends.',
  'network': 'NBC',
  'series_status': 'continuing',
  'tvmaze_id': 42,
  'tvdb_id': 4242,
  'imdb_id': 'tt7654321',
  'monitored': 1,
  'path': '/data/library/series/Great Show',
  'poster_url': '',
  'created_at': '2025-01-01T00:00:00',
};

final fakeEpisode = {
  'id': 11,
  'series_id': 1,
  'season_number': 1,
  'episode_number': 5,
  'title': 'The Middle One',
  'air_date': '2025-01-10',
  'monitored': 1,
  'status': 'missing',
};

final fakeArtist = {
  'id': 1,
  'name': 'Test Artist',
  'sort_name': 'Artist, Test',
  'overview': 'A fictional musician.',
  'mbid': 'aaaa-bbbb',
  'image_url': '',
  'monitored': 1,
  'path': '/data/library/music/Test Artist',
};

final fakeAlbum = {
  'id': 2,
  'artist_id': 1,
  'title': 'Greatest Hits',
  'album_type': 'album',
  'release_date': '2024-05-01',
  'mbid': 'cccc-dddd',
  'monitored': 1,
  'status': 'missing',
};

final fakeRequest = {
  'id': 1,
  'media_type': 'movie',
  'external_id': '123',
  'status': 'pending',
  'title': 'Wanted Flick',
  'year': 2025,
  'poster_url': '',
  'requested_by': 2,
  'requested_by_name': 'alice',
  'approved_by': null,
  'created_at': '2025-01-02T00:00:00',
  'updated_at': '2025-01-02T00:00:00',
};

final fakeQueueItem = {
  'id': 5,
  'torrent_id': 'abc123',
  'name': 'Some.Movie.2024.1080p.WEB-DL.x264-GROUP',
  'size_bytes': 4000000000,
  'state': 'live',
  'indexer': 'Jackett-All',
  'target_title': 'Big Movie (2024)',
  'progress': 0.52,
  'eta_seconds': 18000,
  'live': {'download_speed': 102400, 'eta_seconds': 18000},
};

final fakeHistoryEvent = {
  'id': 7,
  'event_type': 'imported',
  'title': 'Big Movie (2024)',
  'source_title': 'Some.Movie.2024.1080p.WEB-DL.x264-GROUP',
  'download_id': 'abc123',
  'media_type': 'movie',
  'data': '{}',
  'created_at': '2025-01-03T10:00:00',
};

final fakeIndexer = {
  'id': 1,
  'name': 'Jackett-All',
  'url': 'http://localhost:9117/api/v2.0/indexers/all/results/torznab/',
  'api_key': 'secret',
  'enabled': 1,
  'priority': 25,
  'categories': '2000,5000,3000',
};

final fakeCalendarItem = {
  'kind': 'episode',
  'date': '2099-01-10',
  'title': 'The Next One',
  'series_title': 'Great Show',
  'season': 1,
  'episode': 6,
  'status': 'unaired',
  'poster_url': '',
  'series_id': 1,
  'episode_id': 12,
  'monitored': true,
};

final fakeWantedMovie = {
  'kind': 'movie',
  'id': 9,
  'title': 'Missing Movie (2025)',
  'date': '2025-02-01',
  'poster_url': '',
};

final fakeSettings = {
  'general': {'instance_name': 'Finarr', 'host': '0.0.0.0', 'port': 8787},
  'paths': {
    'downloads_dir': '/data/downloads',
    'movies_root': '/data/library/movies',
    'series_root': '/data/library/series',
    'music_root': '/data/library/music',
    'import_mode': 'hardlink'
  },
  'engine': {
    'listen_port': 4242,
    'dht_enabled': true,
    'lsd_enabled': true,
    'download_kbps': 0,
    'upload_kbps': 0,
    'seed_ratio': 0.0
  },
  'metadata': {'tmdb_api_key': ''},
  'automation': {
    'search_interval_min': 30,
    'wanted_search_enabled': true,
    'auto_import': true
  },
  'notifications': {'kind': '', 'url': '', 'token': '', 'priority': 3},
};

/// Build an [AppState] wired to a mocked backend with seeded fields.
AppState fakeAppState({bool loggedIn = true}) {
  SharedPreferences.setMockInitialValues({});
  final client = MockClient((req) async {
    final body = fakeApi(req.url.path, req.url);
    return http.Response(jsonEncode(body), 200,
        headers: {'content-type': 'application/json'});
  });
  final s = AppState(api: Api(client: client), autostart: false);
  if (loggedIn) {
    s.api.token = 'test-token';
    s.user = {
      'id': 1,
      'username': 'admin',
      'display_name': 'Admin',
      'role': 'admin'
    };
  }
  s.booted = true;
  s.torrents = [fakeTorrent];
  s.speeds = {'download': 102400, 'upload': 51200};
  s.movies = [fakeMovie];
  s.seriesList = [fakeSeries];
  s.artists = [fakeArtist];
  s.requests = [fakeRequest];
  s.queue = [fakeQueueItem];
  s.history = [fakeHistoryEvent];
  s.indexers = [fakeIndexer];
  s.clients = [];
  s.users = [
    {'id': 1, 'username': 'admin', 'role': 'admin'}
  ];
  s.calendarItems = [fakeCalendarItem];
  s.wanted = {
    'movies': [fakeWantedMovie],
    'episodes': [],
    'albums': []
  };
  s.settings = fakeSettings;
  s.systemInfo = fakeApi('/api/system/info', Uri());
  s.disk = fakeApi('/api/system/disk', Uri()) as List;
  return s;
}

/// Pump [child] inside an [AppState] provider at [size], light or dark.
Future<void> pumpScreen(
  WidgetTester tester,
  Widget child, {
  Size size = const Size(1280, 800),
  AppState? state,
  Brightness brightness = Brightness.dark,
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  final app = state ?? fakeAppState();
  await tester.pumpWidget(
    ChangeNotifierProvider<AppState>.value(
      value: app,
      child: MaterialApp(
        home: MediaQuery(
          data: MediaQueryData(size: size),
          child: Theme(
            data: brightness == Brightness.dark ? _darkTheme() : _lightTheme(),
            child: Scaffold(body: child),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

// Minimal themes matching lib/theme.dart without pulling network fonts.
ThemeData _darkTheme() => ThemeData(
      brightness: Brightness.dark,
      useMaterial3: true,
      colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF5E5CE6), brightness: Brightness.dark),
    );

ThemeData _lightTheme() => ThemeData(
      brightness: Brightness.light,
      useMaterial3: true,
      colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF5E5CE6), brightness: Brightness.light),
    );
