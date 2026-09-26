import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

void main() => runApp(const LandingApp());

class LandingApp extends StatelessWidget {
  const LandingApp({super.key});

  @override
  Widget build(BuildContext context) {
    const accent = Color(0xFF5E5CE6);
    final scheme = ColorScheme.fromSeed(
      seedColor: accent,
      brightness: Brightness.dark,
      surface: const Color(0xFF0C0C0E),
    );
    return MaterialApp(
      title: 'Finarr',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        brightness: Brightness.dark,
        colorScheme: scheme,
        scaffoldBackgroundColor: const Color(0xFF0C0C0E),
        fontFamily: 'system-ui',
      ),
      home: const LandingPage(),
    );
  }
}

const _features = [
  (
    'Torrent client built in',
    'Magnet links, .torrent files, per-file selection, rate limits, DHT, UPnP and seed-ratio rules, all powered by a real BitTorrent engine.',
    Icons.download_outlined
  ),
  (
    'Movies',
    'Add a movie once and Finarr searches your indexers, picks the best release, downloads it and files it away with clean naming.',
    Icons.movie_outlined
  ),
  (
    'Series',
    'Per-episode and per-season monitoring, season packs, air-date calendar and automatic grabs when new episodes land.',
    Icons.tv_outlined
  ),
  (
    'Music',
    'Artist discographies via MusicBrainz with monitored albums and the same automated pipeline.',
    Icons.album_outlined
  ),
  (
    'Jackett & Prowlarr',
    'Any Torznab endpoint works: point your Jackett or Prowlarr URL at Finarr and every indexer you have is searchable.',
    Icons.hub_outlined
  ),
  (
    'Requests',
    'Seerr-style requests: users ask, you approve, Finarr does the rest. Built for shared servers.',
    Icons.bookmark_add_outlined
  ),
  (
    'One binary',
    'Rust backend with the web UI compiled in. Runs anywhere with a single command: no stack, no env vars, no config files.',
    Icons.bolt_outlined
  ),
  (
    'Every platform',
    'The same Flutter app on web, Android, iOS, Linux, Windows and macOS, pointed at any Finarr server.',
    Icons.devices_outlined
  ),
];

class LandingPage extends StatelessWidget {
  const LandingPage({super.key});

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width > 860;
    return Scaffold(
      body: SingleChildScrollView(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 1080),
            child: Padding(
              padding: EdgeInsets.symmetric(horizontal: wide ? 40 : 20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const SizedBox(height: 24),
                  _Nav(),
                  const SizedBox(height: 96),
                  _Hero(),
                  const SizedBox(height: 96),
                  _FeatureGrid(wide: wide),
                  const SizedBox(height: 96),
                  _Steps(wide: wide),
                  const SizedBox(height: 96),
                  _Footer(),
                  const SizedBox(height: 32),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Nav extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        ClipRRect(
          borderRadius: BorderRadius.circular(7),
          child: Image.asset('assets/icon.png', width: 28, height: 28,
              errorBuilder: (_, _, _) => Container(
                    width: 28,
                    height: 28,
                    decoration: BoxDecoration(
                      color: const Color(0xFF5E5CE6),
                      borderRadius: BorderRadius.circular(7),
                    ),
                    child: const Center(
                        child: Text('F',
                            style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.w800))),
                  )),
        ),
        const SizedBox(width: 10),
        const Text('Finarr',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
        const Spacer(),
        _Link(
            label: 'GitLab',
            url: 'https://gitlab.com/HttpAnimations/finarr'),
      ],
    );
  }
}

class _Hero extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Your media, fully automated.',
            style: TextStyle(
                fontSize: 44,
                fontWeight: FontWeight.w700,
                letterSpacing: -0.5,
                height: 1.1)),
        const SizedBox(height: 20),
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Text(
            'Torrents, indexers, movies, series, music and requests in one '
            'polished app. Finarr replaces qBittorrent + Sonarr + Radarr + '
            'Lidarr + Seerr with a single binary that just works.',
            style: TextStyle(
                fontSize: 17, height: 1.5, color: Colors.white.withValues(alpha: 0.7)),
          ),
        ),
        const SizedBox(height: 32),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            FilledButton.icon(
              onPressed: () => launchUrl(Uri.parse(
                  'https://gitlab.com/HttpAnimations/finarr')),
              icon: const Icon(Icons.code, size: 18),
              label: const Text('Source on GitLab'),
              style: FilledButton.styleFrom(
                backgroundColor: const Color(0xFF5E5CE6),
                foregroundColor: Colors.white,
                padding:
                    const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
              ),
            ),
            OutlinedButton.icon(
              onPressed: () => launchUrl(Uri.parse(
                  'https://github.com/justacalico/finarr/releases')),
              icon: const Icon(Icons.download_outlined, size: 18),
              label: const Text('Download'),
              style: OutlinedButton.styleFrom(
                padding:
                    const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _FeatureGrid extends StatelessWidget {
  final bool wide;
  const _FeatureGrid({required this.wide});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Everything, in one place',
            style: TextStyle(fontSize: 26, fontWeight: FontWeight.w700)),
        const SizedBox(height: 8),
        Text('Every piece of the media stack, redesigned to be reliable.',
            style:
                TextStyle(fontSize: 15, color: Colors.white.withValues(alpha: 0.6))),
        const SizedBox(height: 32),
        GridView.count(
          crossAxisCount: wide ? 2 : 1,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          mainAxisSpacing: 16,
          crossAxisSpacing: 16,
          childAspectRatio: wide ? 3.4 : 3.0,
          children: [
            for (final f in _features)
              _FeatureCard(icon: f.$3, title: f.$1, body: f.$2),
          ],
        ),
      ],
    );
  }
}

class _FeatureCard extends StatelessWidget {
  final IconData icon;
  final String title;
  final String body;
  const _FeatureCard(
      {required this.icon, required this.title, required this.body});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: const Color(0xFF161618),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: const Color(0xFF2C2C2E)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: const Color(0x225E5CE6),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Icon(icon, size: 20, color: const Color(0xFF8B88FF)),
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title,
                    style: const TextStyle(
                        fontSize: 15, fontWeight: FontWeight.w600)),
                const SizedBox(height: 6),
                Text(body,
                    style: TextStyle(
                        fontSize: 13,
                        height: 1.45,
                        color: Colors.white.withValues(alpha: 0.6))),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Steps extends StatelessWidget {
  final bool wide;
  const _Steps({required this.wide});

  @override
  Widget build(BuildContext context) {
    final steps = [
      (
        '1',
        'Run the binary',
        'Download one file and run it. The setup wizard in the web UI does the rest, nothing to configure by hand.'
      ),
      (
        '2',
        'Add your indexers',
        'Paste your Jackett Torznab URL and API key. Every tracker you already use works instantly.'
      ),
      (
        '3',
        'Add media and relax',
        'Monitored titles are searched, downloaded, imported and renamed automatically, on your schedule.'
      ),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Up in three steps',
            style: TextStyle(fontSize: 26, fontWeight: FontWeight.w700)),
        const SizedBox(height: 32),
        if (wide)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              for (final s in steps)
                Expanded(child: _StepCard(n: s.$1, title: s.$2, body: s.$3)),
            ].expand((w) sync* {
              yield w;
              yield const SizedBox(width: 16);
            }).toList()..removeLast(),
          )
        else
          Column(
            children: [
              for (final s in steps)
                Padding(
                  padding: const EdgeInsets.only(bottom: 16),
                  child: _StepCard(n: s.$1, title: s.$2, body: s.$3),
                ),
            ],
          ),
      ],
    );
  }
}

class _StepCard extends StatelessWidget {
  final String n;
  final String title;
  final String body;
  const _StepCard({required this.n, required this.title, required this.body});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: const Color(0xFF161618),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: const Color(0xFF2C2C2E)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(n,
              style: const TextStyle(
                  fontSize: 28,
                  fontWeight: FontWeight.w800,
                  color: Color(0xFF5E5CE6))),
          const SizedBox(height: 8),
          Text(title,
              style: const TextStyle(
                  fontSize: 15, fontWeight: FontWeight.w600)),
          const SizedBox(height: 6),
          Text(body,
              style: TextStyle(
                  fontSize: 13,
                  height: 1.45,
                  color: Colors.white.withValues(alpha: 0.6))),
        ],
      ),
    );
  }
}

class _Footer extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 24),
      decoration: const BoxDecoration(
        border: Border(top: BorderSide(color: Color(0xFF2C2C2E))),
      ),
      child: Row(
        children: [
          Text('Finarr · AGPL-3.0',
              style: TextStyle(
                  fontSize: 13, color: Colors.white.withValues(alpha: 0.5))),
          const Spacer(),
          _Link(
              label: 'GitHub mirror',
              url: 'https://github.com/justacalico/finarr'),
        ],
      ),
    );
  }
}

class _Link extends StatelessWidget {
  final String label;
  final String url;
  const _Link({required this.label, required this.url});

  @override
  Widget build(BuildContext context) {
    return TextButton(
      onPressed: () => launchUrl(Uri.parse(url)),
      child: Text(label),
    );
  }
}
