import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../widgets.dart';

/// Everything missing: wanted movies, aired episodes, released albums.
class WantedView extends StatelessWidget {
  const WantedView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    final movies = (s.wanted['movies'] ?? []) as List;
    final episodes = (s.wanted['episodes'] ?? []) as List;
    final albums = (s.wanted['albums'] ?? []) as List;
    final empty = movies.isEmpty && episodes.isEmpty && albums.isEmpty;

    return Scaffold(
      body: RefreshIndicator(
        onRefresh: s.refreshWanted,
        child: empty
            ? const EmptyState(Icons.check_circle_outline, 'All caught up',
                message: 'Nothing wanted right now. Finarr keeps looking automatically.')
            : ListView(
                padding: EdgeInsets.all(width >= 900 ? 32 : 16),
                children: [
                  const SectionHeader('Wanted',
                      subtitle: 'Monitored media that is missing'),
                  if (movies.isNotEmpty) ...[
                    _GroupLabel('Movies · ${movies.length}'),
                    Card(
                      margin: const EdgeInsets.only(bottom: 20),
                      child: Column(children: [
                        for (final m in movies) _WantedRow(item: m),
                      ]),
                    ),
                  ],
                  if (episodes.isNotEmpty) ...[
                    _GroupLabel('Episodes · ${episodes.length}'),
                    Card(
                      margin: const EdgeInsets.only(bottom: 20),
                      child: Column(children: [
                        for (final e in episodes) _WantedRow(item: e),
                      ]),
                    ),
                  ],
                  if (albums.isNotEmpty) ...[
                    _GroupLabel('Albums · ${albums.length}'),
                    Card(
                      child: Column(children: [
                        for (final a in albums) _WantedRow(item: a),
                      ]),
                    ),
                  ],
                ],
              ),
      ),
    );
  }
}

class _GroupLabel extends StatelessWidget {
  final String text;
  const _GroupLabel(this.text);
  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(left: 4, bottom: 8),
        child: Text(text,
            style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w700,
                color: Theme.of(context).hintColor)),
      );
}

class _WantedRow extends StatelessWidget {
  final Map<String, dynamic> item;
  const _WantedRow({required this.item});

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Poster(url: item['poster_url'], title: item['title'] ?? '', width: 36),
      title: Text(item['title'] ?? '',
          maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: Text(
        '${item['date'] != null ? fmtDate(item['date']) : ''} ${item['episode_title'] ?? ''}'
            .trim(),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor),
      ),
      trailing: IconButton(
        tooltip: 'Search releases',
        icon: const Icon(Icons.search, size: 18),
        onPressed: () => _search(context),
      ),
    );
  }

  void _search(BuildContext context) {
    final s = context.read<AppState>();
    final item = this.item;
    // Route to the interactive search endpoint for this item kind.
    final (path, body) = switch (item['kind']) {
      'movie' => (
          '/api/movies/${item['id']}/search',
          {'media_type': 'movie', 'movie_id': item['id']}
        ),
      'episode' => (
          '/api/episodes/${item['id']}/search',
          {'media_type': 'series', 'episode_ids': [item['id']]}
        ),
      'album' => (
          '/api/albums/${item['id']}/search',
          {'media_type': 'album', 'album_id': item['id']}
        ),
      _ => ('', <String, dynamic>{}),
    };
    if (path.isEmpty) return;
    showSearchSheet(context, s, item['title'] ?? '', path, body);
  }
}

void showSearchSheet(BuildContext context, AppState s, String title,
    String searchPath, Map<String, dynamic> grabBody) {
  // Reuse the shared sheet defined in release_search.dart without
  // circular imports: keep it minimal here.
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (_) => FractionallySizedBox(
      heightFactor: 0.9,
      child: _InlineSearch(
          title: title, searchPath: searchPath, grabBody: grabBody),
    ),
  );
}

class _InlineSearch extends StatefulWidget {
  final String title;
  final String searchPath;
  final Map<String, dynamic> grabBody;
  const _InlineSearch(
      {required this.title, required this.searchPath, required this.grabBody});

  @override
  State<_InlineSearch> createState() => _InlineSearchState();
}

class _InlineSearchState extends State<_InlineSearch> {
  List<dynamic>? _releases;

  @override
  void initState() {
    super.initState();
    _go();
  }

  Future<void> _go() async {
    final r = await context.read<AppState>().api.post(widget.searchPath);
    if (mounted) setState(() => _releases = r['releases'] ?? []);
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(16),
          child: Row(children: [
            Expanded(
                child: Text(widget.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                        fontSize: 16, fontWeight: FontWeight.w600))),
            IconButton(
                onPressed: () => Navigator.pop(context),
                icon: const Icon(Icons.close)),
          ]),
        ),
        const Divider(height: 1),
        Expanded(
          child: _releases == null
              ? const Center(child: CircularProgressIndicator())
              : _releases!.isEmpty
                  ? const Center(child: Text('No releases found'))
                  : ListView.builder(
                      itemCount: _releases!.length,
                      itemBuilder: (context, i) {
                        final r = _releases![i];
                        return ListTile(
                          title: Text(r['title'] ?? '',
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(fontSize: 13)),
                          subtitle: Text(
                              '${fmtBytes(r['size_bytes'] ?? 0)} · ${r['seeders']}↑ · ${r['indexer']}',
                              style: const TextStyle(fontSize: 11)),
                          trailing: FilledButton.tonal(
                            onPressed: () async {
                              await context.read<AppState>().api.post(
                                  '/api/releases/grab', {
                                ...widget.grabBody,
                                'title': r['title'],
                                'download_url': r['download_url'],
                                'info_hash': r['info_hash'],
                                'size_bytes': r['size_bytes'],
                                'seeders': r['seeders'],
                                'peers': r['peers'],
                                'indexer': r['indexer'],
                              });
                              if (context.mounted) {
                                snack(context, 'Grabbed');
                                Navigator.pop(context);
                              }
                            },
                            child: const Text('Grab'),
                          ),
                        );
                      },
                    ),
        ),
      ],
    );
  }
}
