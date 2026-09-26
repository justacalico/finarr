import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../widgets.dart';
import 'release_search.dart';

class ArtistDetailView extends StatefulWidget {
  final int id;
  const ArtistDetailView({super.key, required this.id});

  @override
  State<ArtistDetailView> createState() => _ArtistDetailViewState();
}

class _ArtistDetailViewState extends State<ArtistDetailView> {
  Map<String, dynamic>? _artist;
  List<dynamic> _albums = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final r = await context
          .read<AppState>()
          .api
          .get('/api/artists/${widget.id}');
      if (!mounted) return;
      setState(() {
        _artist = r['artist'];
        _albums = r['albums'] ?? [];
      });
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final a = _artist;
    final wide = MediaQuery.sizeOf(context).width >= 800;
    return Scaffold(
      appBar: AppBar(
        title: Text(a?['name'] ?? ''),
        actions: [
          if (a != null)
            IconButton(
              tooltip: 'Delete',
              icon: const Icon(Icons.delete_outline),
              onPressed: _delete,
            ),
        ],
      ),
      body: a == null
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: EdgeInsets.all(wide ? 32 : 16),
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: wide ? 160 : 110,
                      child: Poster(
                          url: a['image_url'],
                          title: a['name'] ?? '',
                          aspect: 1),
                    ),
                    const SizedBox(width: 20),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(a['name'] ?? '',
                              style: const TextStyle(
                                  fontSize: 22, fontWeight: FontWeight.w700)),
                          const SizedBox(height: 8),
                          if ((a['overview'] ?? '').isNotEmpty)
                            Text(a['overview'],
                                style: TextStyle(
                                    fontSize: 13,
                                    color: Theme.of(context)
                                        .textTheme
                                        .bodySmall
                                        ?.color)),
                          const SizedBox(height: 16),
                          OutlinedButton.icon(
                            onPressed: _toggleMonitored,
                            icon: Icon(
                                a['monitored'] == 1
                                    ? Icons.visibility_off_outlined
                                    : Icons.visibility_outlined,
                                size: 18),
                            label: Text(a['monitored'] == 1
                                ? 'Unmonitor'
                                : 'Monitor'),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 28),
                CardSection(
                  title: 'Albums (${_albums.length})',
                  children: [
                    for (final album in _albums) _albumRow(album),
                  ],
                ),
              ],
            ),
    );
  }

  Widget _albumRow(Map<String, dynamic> album) {
    final monitored = album['monitored'] == 1;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          SizedBox(
            width: 24,
            child: Checkbox(
              value: monitored,
              onChanged: (v) async {
                await context
                    .read<AppState>()
                    .api
                    .patch('/api/albums/${album['id']}', {'monitored': v ?? false});
                _load();
              },
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(album['title'] ?? '',
                    style: const TextStyle(fontSize: 14)),
                Text(
                  '${album['album_type'] ?? 'album'}${album['release_date'] != null ? ' · ${fmtDate(album['release_date'])}' : ''}',
                  style: TextStyle(
                      fontSize: 11, color: Theme.of(context).hintColor),
                ),
              ],
            ),
          ),
          StatusChip(album['status'] ?? ''),
          if (album['status'] == 'missing')
            IconButton(
              tooltip: 'Search releases',
              icon: const Icon(Icons.search, size: 18),
              onPressed: () => ReleaseSearchSheet.show(
                context,
                title: album['title'] ?? '',
                searchPath: '/api/albums/${album['id']}/search',
                grabBody: {
                  'media_type': 'album',
                  'album_id': album['id'],
                },
              ).then((_) => _load()),
            ),
        ],
      ),
    );
  }

  Future<void> _toggleMonitored() async {
    final a = _artist!;
    await context.read<AppState>().api.put('/api/artists/${widget.id}', {
      'monitored': a['monitored'] != 1
    });
    _load();
  }

  Future<void> _delete() async {
    final files = await confirm(context,
        title: 'Delete artist',
        message: 'Also delete imported files from disk?',
        confirmLabel: 'Delete files',
        destructive: true);
    if (!mounted) return;
    if (files) {
      await context
          .read<AppState>()
          .api
          .delete('/api/artists/${widget.id}', {'delete_files': 'true'});
    } else {
      final ok = await confirm(context,
          title: 'Remove from library?',
          message: 'Files on disk are kept.',
          confirmLabel: 'Remove');
      if (!ok || !mounted) return;
      await context.read<AppState>().api.delete('/api/artists/${widget.id}');
    }
    if (mounted) {
      context.read<AppState>().refreshLibraries();
      Navigator.of(context).pop();
    }
  }
}
