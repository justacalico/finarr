import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';
import 'release_search.dart';

class SeriesDetailView extends StatefulWidget {
  final int id;
  const SeriesDetailView({super.key, required this.id});

  @override
  State<SeriesDetailView> createState() => _SeriesDetailViewState();
}

class _SeriesDetailViewState extends State<SeriesDetailView> {
  Map<String, dynamic>? _series;
  List<dynamic> _episodes = [];
  List<dynamic> _seasons = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final api = context.read<AppState>().api;
      final r = await api.get('/api/series/${widget.id}');
      final e = await api.get('/api/series/${widget.id}/episodes');
      setState(() {
        _series = r['series'];
        _seasons = e['seasons'] ?? [];
        _episodes = e['episodes'] ?? [];
      });
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final s = _series;
    final wide = MediaQuery.sizeOf(context).width >= 800;
    return Scaffold(
      appBar: AppBar(
        title: Text(s?['title'] ?? ''),
        actions: [
          if (s != null) ...[
            IconButton(
              tooltip: 'Refresh metadata',
              icon: const Icon(Icons.sync),
              onPressed: () async {
                await context
                    .read<AppState>()
                    .api
                    .post('/api/series/${widget.id}/refresh');
                _load();
                if (context.mounted) {
                  snack(context, 'Metadata refreshed');
                }
              },
            ),
            IconButton(
              tooltip: 'Delete',
              icon: const Icon(Icons.delete_outline),
              onPressed: _delete,
            ),
          ],
        ],
      ),
      body: s == null
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: EdgeInsets.all(wide ? 32 : 16),
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: wide ? 200 : 130,
                      child: Poster(url: s['poster_url'], title: s['title'] ?? ''),
                    ),
                    const SizedBox(width: 20),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            '${s['title'] ?? ''}${s['year'] != null ? ' (${s['year']})' : ''}',
                            style: const TextStyle(
                                fontSize: 22, fontWeight: FontWeight.w700),
                          ),
                          const SizedBox(height: 6),
                          Wrap(
                            spacing: 8,
                            runSpacing: 6,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: [
                              StatusChip(s['series_status'] ?? ''),
                              if (s['network'] != null)
                                Text('${s['network']}',
                                    style: TextStyle(
                                        fontSize: 12,
                                        color: Theme.of(context).hintColor)),
                            ],
                          ),
                          const SizedBox(height: 12),
                          if ((s['overview'] ?? '').isNotEmpty)
                            Text(s['overview'],
                                maxLines: 6,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                    fontSize: 13,
                                    height: 1.5,
                                    color: Theme.of(context)
                                        .textTheme
                                        .bodySmall
                                        ?.color)),
                          const SizedBox(height: 16),
                          Wrap(
                            spacing: 8,
                            runSpacing: 8,
                            children: [
                              FilledButton.icon(
                                onPressed: () => ReleaseSearchSheet.show(
                                  context,
                                  title: s['title'] ?? '',
                                  searchPath: '/api/series/${widget.id}/search',
                                  grabBody: {
                                    'media_type': 'series',
                                    'episode_ids': _missingEpisodes(),
                                  },
                                ).then((_) => _load()),
                                icon: const Icon(Icons.search, size: 18),
                                label: const Text('Search releases'),
                              ),
                              OutlinedButton.icon(
                                onPressed: _toggleMonitored,
                                icon: Icon(
                                    s['monitored'] == 1
                                        ? Icons.visibility_off_outlined
                                        : Icons.visibility_outlined,
                                    size: 18),
                                label: Text(s['monitored'] == 1
                                    ? 'Unmonitor'
                                    : 'Monitor'),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 28),
                for (final season in _orderedSeasons()) _seasonBlock(season),
              ],
            ),
    );
  }

  List<int> _missingEpisodes() => _episodes
      .where((e) => e['status'] == 'missing' && e['monitored'] == 1)
      .map((e) => e['id'] as int)
      .toList();

  List<int> _orderedSeasons() {
    final nums = _episodes
        .map((e) => e['season_number'] as int)
        .toSet()
        .toList();
    nums.sort((a, b) => b.compareTo(a)); // newest season first
    return nums;
  }

  Map<String, dynamic>? _seasonRow(int number) {
    for (final s in _seasons) {
      if (s['season_number'] == number) return s;
    }
    return null;
  }

  Widget _seasonBlock(int number) {
    final season = _seasonRow(number);
    final eps = _episodes
        .where((e) => e['season_number'] == number)
        .toList()
      ..sort((a, b) => (a['episode_number'] ?? 0).compareTo(b['episode_number'] ?? 0));
    final monitored = season?['monitored'] == 1;
    final missing = eps.where((e) => e['status'] == 'missing').length;
    return CardSection(
      title: number == 0 ? 'Specials' : 'Season $number',
      children: [
        Row(
          children: [
            Switch(
              value: monitored,
              onChanged: (v) async {
                if (season == null) return;
                await context
                    .read<AppState>()
                    .api
                    .put('/api/seasons/${season['id']}/monitor', {'monitored': v});
                _load();
              },
            ),
            Text('Monitor season',
                style: TextStyle(color: Theme.of(context).hintColor, fontSize: 13)),
            const Spacer(),
            if (missing > 0)
              Text('$missing missing',
                  style: const TextStyle(color: F.warn, fontSize: 12)),
          ],
        ),
        const Divider(height: 16),
        for (final ep in eps) _episodeRow(ep),
      ],
    );
  }

  Widget _episodeRow(Map<String, dynamic> ep) {
    final monitored = ep['monitored'] == 1;
    final aired = ep['air_date'] == null ||
        ep['air_date'].compareTo(DateTime.now().toIso8601String().substring(0, 10)) <= 0;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Checkbox(
            value: monitored,
            onChanged: (v) async {
              await context
                  .read<AppState>()
                  .api
                  .patch('/api/episodes/${ep['id']}', {'monitored': v ?? false});
              _load();
            },
          ),
          SizedBox(
            width: 44,
            child: Text('E${ep['episode_number'].toString().padLeft(2, '0')}',
                style: TextStyle(
                    fontFamily: 'monospace',
                    fontSize: 12,
                    color: Theme.of(context).hintColor)),
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(ep['title'] ?? 'TBA',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                        fontSize: 13,
                        color: aired ? null : Theme.of(context).hintColor)),
                if (ep['air_date'] != null)
                  Text(fmtDate(ep['air_date']),
                      style: TextStyle(
                          fontSize: 11, color: Theme.of(context).hintColor)),
              ],
            ),
          ),
          StatusChip(ep['status'] ?? ''),
          if (aired && ep['status'] == 'missing')
            IconButton(
              tooltip: 'Search releases',
              icon: const Icon(Icons.search, size: 18),
              onPressed: () => ReleaseSearchSheet.show(
                context,
                title:
                    'S${ep['season_number'].toString().padLeft(2, '0')}E${ep['episode_number'].toString().padLeft(2, '0')} — ${ep['title']}',
                searchPath: '/api/episodes/${ep['id']}/search',
                grabBody: {
                  'media_type': 'series',
                  'episode_ids': [ep['id']],
                },
              ).then((_) => _load()),
            ),
        ],
      ),
    );
  }

  Future<void> _toggleMonitored() async {
    final s = _series!;
    await context.read<AppState>().api.put('/api/series/${widget.id}', {
      'monitored': s['monitored'] != 1
    });
    _load();
  }

  Future<void> _delete() async {
    final files = await confirm(context,
        title: 'Delete series',
        message: 'Also delete imported files from disk?',
        confirmLabel: 'Delete files',
        destructive: true);
    if (!mounted) return;
    if (files) {
      await context
          .read<AppState>()
          .api
          .delete('/api/series/${widget.id}', {'delete_files': 'true'});
    } else {
      final ok = await confirm(context,
          title: 'Remove from library?',
          message: 'Files on disk are kept.',
          confirmLabel: 'Remove');
      if (!ok || !mounted) return;
      await context.read<AppState>().api.delete('/api/series/${widget.id}');
    }
    if (mounted) {
      context.read<AppState>().refreshLibraries();
      Navigator.of(context).pop();
    }
  }
}
