import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../widgets.dart';
import 'release_search.dart';

class MovieDetailView extends StatefulWidget {
  final int id;
  const MovieDetailView({super.key, required this.id});

  @override
  State<MovieDetailView> createState() => _MovieDetailViewState();
}

class _MovieDetailViewState extends State<MovieDetailView> {
  Map<String, dynamic>? _movie;
  List<dynamic> _files = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final r = await context.read<AppState>().api.get('/api/movies/${widget.id}');
      setState(() {
        _movie = r['movie'];
        _files = r['files'] ?? [];
      });
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final m = _movie;
    final wide = MediaQuery.sizeOf(context).width >= 800;
    return Scaffold(
      appBar: AppBar(
        title: Text(m?['title'] ?? ''),
        actions: [
          if (m != null)
            IconButton(
              tooltip: 'Delete',
              icon: const Icon(Icons.delete_outline),
              onPressed: _delete,
            ),
        ],
      ),
      body: m == null
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: EdgeInsets.all(wide ? 32 : 16),
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: wide ? 200 : 130,
                      child: Poster(url: m['poster_url'], title: m['title'] ?? ''),
                    ),
                    const SizedBox(width: 20),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            '${m['title'] ?? ''}${m['year'] != null ? ' (${m['year']})' : ''}',
                            style: const TextStyle(
                                fontSize: 22, fontWeight: FontWeight.w700),
                          ),
                          const SizedBox(height: 6),
                          Wrap(
                            spacing: 8,
                            runSpacing: 6,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: [
                              StatusChip(m['status'] ?? ''),
                              if (m['runtime_min'] != null)
                                _Dim('${m['runtime_min']} min'),
                              if ((m['genres'] ?? '').isNotEmpty)
                                _Dim(_genreList(m['genres'])),
                            ],
                          ),
                          const SizedBox(height: 12),
                          if ((m['overview'] ?? '').isNotEmpty)
                            Text(m['overview'],
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
                                  title: m['title'] ?? '',
                                  searchPath: '/api/movies/${widget.id}/search',
                                  grabBody: {
                                    'media_type': 'movie',
                                    'movie_id': widget.id,
                                  },
                                ).then((_) => _load()),
                                icon: const Icon(Icons.search, size: 18),
                                label: const Text('Search releases'),
                              ),
                              OutlinedButton.icon(
                                onPressed: _toggleMonitored,
                                icon: Icon(
                                    m['monitored'] == 1
                                        ? Icons.visibility_off_outlined
                                        : Icons.visibility_outlined,
                                    size: 18),
                                label: Text(m['monitored'] == 1
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
                CardSection(
                  title: 'Files',
                  children: [
                    if (_files.isEmpty)
                      Text('No files imported yet',
                          style: TextStyle(color: Theme.of(context).hintColor)),
                    for (final f in _files)
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 6),
                        child: Row(
                          children: [
                            const Icon(Icons.videocam_outlined, size: 18),
                            const SizedBox(width: 10),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(f['path'] ?? '',
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: const TextStyle(fontSize: 13)),
                                  Text(
                                    '${f['quality'] ?? ''} · ${fmtBytes(f['size_bytes'] ?? 0)}',
                                    style: TextStyle(
                                        fontSize: 11,
                                        color: Theme.of(context).hintColor),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),
                  ],
                ),
              ],
            ),
    );
  }

  String _genreList(String raw) {
    try {
      final list = List<dynamic>.from(raw.isEmpty ? [] : (raw.startsWith('[') ? jsonDecode(raw) : []));
      return list.take(3).join(', ');
    } catch (_) {
      return '';
    }
  }

  Future<void> _toggleMonitored() async {
    final m = _movie!;
    await context.read<AppState>().api.put('/api/movies/${widget.id}', {
      'monitored': m['monitored'] != 1
    });
    _load();
  }

  Future<void> _delete() async {
    final files = await confirm(context,
        title: 'Delete movie',
        message: 'Also delete the imported files from disk?',
        confirmLabel: 'Delete files',
        destructive: true);
    if (!mounted) return;
    if (files) {
      await context.read<AppState>().api
          .delete('/api/movies/${widget.id}', {'delete_files': 'true'});
    } else {
      final ok = await confirm(context,
          title: 'Remove from library?',
          message: 'Files on disk are kept.',
          confirmLabel: 'Remove');
      if (!ok || !mounted) return;
      await context.read<AppState>().api
          .delete('/api/movies/${widget.id}');
    }
    if (mounted) {
      context.read<AppState>().refreshLibraries();
      Navigator.of(context).pop();
    }
  }
}

class _Dim extends StatelessWidget {
  final String text;
  const _Dim(this.text);
  @override
  Widget build(BuildContext context) =>
      Text(text, style: TextStyle(fontSize: 12, color: Theme.of(context).hintColor));
}
