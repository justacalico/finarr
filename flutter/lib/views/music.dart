import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

class MusicView extends StatelessWidget {
  const MusicView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    return Scaffold(
      body: RefreshIndicator(
        onRefresh: s.refreshLibraries,
        child: ListView(
          padding: EdgeInsets.all(width >= 900 ? 32 : 16),
          children: [
            SectionHeader(
              'Music',
              subtitle: '${s.artists.length} artists',
              trailing: FilledButton.icon(
                onPressed: () => _addArtist(context),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('Add Artist'),
              ),
            ),
            if (s.artists.isEmpty)
              EmptyState(
                Icons.album_outlined,
                'No artists yet',
                message: 'Add an artist and Finarr watches their discography.',
                action: FilledButton.icon(
                  onPressed: () => _addArtist(context),
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Add Artist'),
                ),
              )
            else
              Card(
                child: Column(
                  children: [
                    for (final a in s.artists)
                      ListTile(
                        leading: Poster(
                            url: a['image_url'],
                            title: a['name'] ?? '',
                            aspect: 1,
                            width: 40),
                        title: Text(a['name'] ?? ''),
                        subtitle: a['overview'] != ''
                            ? Text(a['overview'],
                                maxLines: 1, overflow: TextOverflow.ellipsis)
                            : null,
                        trailing: StatusChip(
                            a['monitored'] == 1 ? 'imported' : 'missing'),
                        onTap: () => context.push('/artist/${a['id']}'),
                      ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _addArtist(BuildContext context) async {
    final s = context.read<AppState>();
    final picked = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => const _ArtistSearchDialog(),
    );
    if (picked == null || !context.mounted) return;
    try {
      await s.api.post('/api/artists', {'mbid': picked['mbid']});
      await s.refreshLibraries();
      if (context.mounted) snack(context, 'Added ${picked['name']}');
    } catch (e) {
      if (context.mounted) snack(context, '$e', error: true);
    }
  }
}

class _ArtistSearchDialog extends StatefulWidget {
  const _ArtistSearchDialog();

  @override
  State<_ArtistSearchDialog> createState() => _ArtistSearchDialogState();
}

class _ArtistSearchDialogState extends State<_ArtistSearchDialog> {
  final _q = TextEditingController();
  List<dynamic>? _results;
  bool _busy = false;
  String? _error;

  Future<void> _search() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final r = await context
          .read<AppState>()
          .api
          .get('/api/lookup/artists', {'q': _q.text.trim()});
      setState(() => _results = r['results'] ?? []);
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add artist'),
      content: SizedBox(
        width: 480,
        height: 420,
        child: Column(
          children: [
            TextField(
              controller: _q,
              autofocus: true,
              decoration: InputDecoration(
                hintText: 'Search MusicBrainz...',
                suffixIcon: IconButton(
                    onPressed: _search, icon: const Icon(Icons.search)),
              ),
              onSubmitted: (_) => _search(),
            ),
            const SizedBox(height: 12),
            Expanded(child: _body()),
          ],
        ),
      ),
      actions: [
        TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel')),
      ],
    );
  }

  Widget _body() {
    if (_busy) return const Center(child: CircularProgressIndicator());
    if (_error != null) {
      return Center(
          child: Text(_error!, style: const TextStyle(color: F.bad, fontSize: 13)));
    }
    if (_results == null) {
      return Center(
          child: Text('Search for an artist to add',
              style: TextStyle(color: Theme.of(context).hintColor)));
    }
    if (_results!.isEmpty) {
      return const Center(child: Text('No results'));
    }
    return ListView.builder(
      itemCount: _results!.length,
      itemBuilder: (context, i) {
        final a = _results![i];
        return ListTile(
          leading: Poster(
              url: a['image_url'], title: a['name'] ?? '', aspect: 1, width: 34),
          title: Text(a['name'] ?? '', maxLines: 1, overflow: TextOverflow.ellipsis),
          subtitle: a['overview'] != ''
              ? Text(a['overview'],
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 11))
              : null,
          onTap: () => Navigator.pop(context, a),
        );
      },
    );
  }
}
