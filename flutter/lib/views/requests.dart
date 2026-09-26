import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// Seerr-style requests: anyone can ask, admins approve or decline.
class RequestsView extends StatelessWidget {
  const RequestsView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    return Scaffold(
      body: RefreshIndicator(
        onRefresh: s.refreshRequests,
        child: ListView(
          padding: EdgeInsets.all(width >= 900 ? 32 : 16),
          children: [
            SectionHeader(
              'Requests',
              subtitle: '${s.requests.length} total',
              trailing: FilledButton.icon(
                onPressed: () => _requestFlow(context),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('Request'),
              ),
            ),
            if (s.requests.isEmpty)
              EmptyState(
                Icons.bookmark_add_outlined,
                'No requests',
                message: 'Request a movie, series or artist to get started.',
                action: FilledButton.icon(
                  onPressed: () => _requestFlow(context),
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Request'),
                ),
              )
            else
              Card(
                child: Column(
                  children: [
                    for (final r in s.requests) _RequestTile(request: r),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _requestFlow(BuildContext context) async {
    final type = await showModalBottomSheet<String>(
      context: context,
      showDragHandle: true,
      builder: (c) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Padding(
              padding: EdgeInsets.all(8),
              child: Text('What do you want to request?',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            ),
            ListTile(
                leading: const Icon(Icons.movie_outlined),
                title: const Text('Movie'),
                onTap: () => Navigator.pop(c, 'movie')),
            ListTile(
                leading: const Icon(Icons.tv_outlined),
                title: const Text('Series'),
                onTap: () => Navigator.pop(c, 'series')),
            ListTile(
                leading: const Icon(Icons.album_outlined),
                title: const Text('Artist'),
                onTap: () => Navigator.pop(c, 'artist')),
          ],
        ),
      ),
    );
    if (type == null || !context.mounted) return;
    final picked = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => _RequestSearchDialog(kind: type),
    );
    if (picked == null || !context.mounted) return;
    final externalId = switch (type) {
      'movie' => '${picked['tmdb_id']}',
      'series' => '${picked['tvmaze_id']}',
      _ => '${picked['mbid']}',
    };
    try {
      final s = context.read<AppState>();
      await s.api.post('/api/requests', {
        'media_type': type,
        'external_id': externalId,
      });
      await s.refreshRequests();
      if (context.mounted) snack(context, 'Request submitted');
    } catch (e) {
      if (context.mounted) snack(context, '$e', error: true);
    }
  }
}

class _RequestTile extends StatelessWidget {
  final Map<String, dynamic> request;
  const _RequestTile({required this.request});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final pending = request['status'] == 'pending';
    
    return ListTile(
      leading: Poster(
          url: request['poster_url'], title: request['title'] ?? '', width: 40),
      title: Text(
          '${request['title']}${request['year'] != null ? ' (${request['year']})' : ''}'),
      subtitle: Text(
        '${request['media_type']} · requested by ${request['requested_by_name']} · ${fmtDate(request['created_at'])}',
        style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor),
      ),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          StatusChip(request['status'] ?? ''),
          if (pending && s.isAdmin) ...[
            const SizedBox(width: 6),
            IconButton(
              tooltip: 'Approve',
              icon: const Icon(Icons.check_circle_outline, color: F.ok),
              onPressed: () async {
                final s = context.read<AppState>();
                try {
                  await s.api
                      .post('/api/requests/${request['id']}/approve');
                } catch (e) {
                  if (context.mounted) snack(context, '$e', error: true);
                }
                s.refreshRequests();
              },
            ),
            IconButton(
              tooltip: 'Decline',
              icon: const Icon(Icons.cancel_outlined, color: F.bad),
              onPressed: () async {
                final s = context.read<AppState>();
                try {
                  await s.api
                      .post('/api/requests/${request['id']}/decline');
                } catch (e) {
                  if (context.mounted) snack(context, '$e', error: true);
                }
                s.refreshRequests();
              },
            ),
          ],
          if (pending || s.isAdmin)
            IconButton(
              tooltip: 'Delete',
              icon: const Icon(Icons.delete_outline, size: 18),
              onPressed: () async {
                final s = context.read<AppState>();
                try {
                  await s.api
                      .delete('/api/requests/${request['id']}');
                } catch (e) {
                  if (context.mounted) snack(context, '$e', error: true);
                }
                s.refreshRequests();
              },
            ),
        ],
      ),
    );
  }
}

class _RequestSearchDialog extends StatefulWidget {
  final String kind;
  const _RequestSearchDialog({required this.kind});

  @override
  State<_RequestSearchDialog> createState() => _RequestSearchDialogState();
}

class _RequestSearchDialogState extends State<_RequestSearchDialog> {
  final _q = TextEditingController();
  List<dynamic>? _results;
  bool _busy = false;
  String? _error;

  String get _endpoint => switch (widget.kind) {
        'movie' => '/api/lookup/movies',
        'series' => '/api/lookup/series',
        _ => '/api/lookup/artists',
      };

  Future<void> _search() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final r = await context
          .read<AppState>()
          .api
          .get(_endpoint, {'q': _q.text.trim()});
      setState(() => _results = r['results'] ?? []);
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final label = switch (widget.kind) {
      'movie' => 'Movie',
      'series' => 'Series',
      _ => 'Artist',
    };
    return AlertDialog(
      title: Text('Request $label'),
      content: SizedBox(
        width: 480,
        height: 420,
        child: Column(
          children: [
            TextField(
              controller: _q,
              autofocus: true,
              decoration: InputDecoration(
                hintText: 'Search...',
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
          child: Text('Search to request',
              style: TextStyle(color: Theme.of(context).hintColor)));
    }
    if (_results!.isEmpty) return const Center(child: Text('No results'));
    return ListView.builder(
      itemCount: _results!.length,
      itemBuilder: (context, i) {
        final r = _results![i];
        return ListTile(
          leading: Poster(
              url: r['poster_url'] ?? r['image_url'],
              title: r['title'] ?? r['name'] ?? '',
              aspect: widget.kind == 'artist' ? 1 : 2 / 3,
              width: 34),
          title: Text(r['title'] ?? r['name'] ?? '',
              maxLines: 1, overflow: TextOverflow.ellipsis),
          subtitle: Text(
            '${r['year'] ?? ''} ${r['overview'] ?? ''}',
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(fontSize: 11),
          ),
          onTap: () => Navigator.pop(context, r),
        );
      },
    );
  }
}
