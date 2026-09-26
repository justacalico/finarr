import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import 'app_state.dart';
import 'theme.dart';
import 'views/login.dart';
import 'views/setup.dart';
import 'views/shell.dart';
import 'views/movie_detail.dart';
import 'views/series_detail.dart';
import 'views/artist_detail.dart';

void main() {
  runApp(
    ChangeNotifierProvider(
      create: (_) => AppState(),
      child: const FinarrApp(),
    ),
  );
}

class FinarrApp extends StatefulWidget {
  const FinarrApp({super.key});

  @override
  State<FinarrApp> createState() => _FinarrAppState();
}

class _FinarrAppState extends State<FinarrApp> {
  late final GoRouter _router;

  @override
  void initState() {
    super.initState();
    final appState = context.read<AppState>();
    _router = GoRouter(
      initialLocation: '/',
      refreshListenable: appState,
      redirect: (context, state) {
        final s = context.read<AppState>();
        if (!s.booted) return null;
        final loc = state.matchedLocation;
        if (s.setupRequired) {
          return loc == '/setup' ? null : '/setup';
        }
        if (!s.loggedIn) {
          return loc == '/login' ? null : '/login';
        }
        if (loc == '/login' || loc == '/setup') return '/';
        return null;
      },
      routes: [
        GoRoute(path: '/setup', builder: (_, _) => const SetupView()),
        GoRoute(path: '/login', builder: (_, _) => const LoginView()),
        GoRoute(path: '/', builder: (_, _) => const ShellView()),
        GoRoute(
            path: '/movie/:id',
            builder: (_, st) =>
                MovieDetailView(id: int.parse(st.pathParameters['id']!))),
        GoRoute(
            path: '/series/:id',
            builder: (_, st) =>
                SeriesDetailView(id: int.parse(st.pathParameters['id']!))),
        GoRoute(
            path: '/artist/:id',
            builder: (_, st) =>
                ArtistDetailView(id: int.parse(st.pathParameters['id']!))),
        GoRoute(
            path: '/settings',
            builder: (_, _) => const ShellView(initialIndex: 9)),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    return MaterialApp.router(
      title: s.instanceName,
      debugShowCheckedModeBanner: false,
      theme: F.light(),
      darkTheme: F.dark(),
      themeMode: switch (s.themeMode) {
        'light' => ThemeMode.light,
        'dark' => ThemeMode.dark,
        _ => ThemeMode.system,
      },
      routerConfig: _router,
    );
  }
}
