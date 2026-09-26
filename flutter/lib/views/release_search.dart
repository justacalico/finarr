import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// Interactive release search: queries indexers, shows scored results,
/// one tap sends the pick to the download client.
class ReleaseSearchSheet extends StatefulWidget {
  final String title;
  final String searchPath; // e.g. /api/episodes/12/search
  final Map<String, dynamic> grabBody; // media_type + ref ids

  const ReleaseSearchSheet({
    super.key,
    required this.title,
    required this.searchPath,
    required this.grabBody,
  });

  static Future<void> show(BuildContext context,
      {required String title,
      required String searchPath,
      required Map<String, dynamic> grabBody}) {
    return showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (_) => FractionallySizedBox(
        heightFactor: 0.9,
        child: ReleaseSearchSheet(
          title: title,
          searchPath: searchPath,
          grabBody: grabBody,
        ),
      ),
    );
  }

  @override
  State<ReleaseSearchSheet> createState() => _ReleaseSearchSheetState();
}

class _ReleaseSearchSheetState extends State<ReleaseSearchSheet> {
  List<dynamic>? _releases;
  String? _error;
  String? _grabbing;

  @override
  void initState() {
    super.initState();
    _search();
  }

  Future<void> _search() async {
    setState(() {
      _releases = null;
      _error = null;
    });
    try {
      final r = await context.read<AppState>().api.post(widget.searchPath);
      setState(() => _releases = r['releases'] ?? []);
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _grab(Map<String, dynamic> rel) async {
    setState(() => _grabbing = rel['guid'] ?? rel['title']);
    try {
      await context.read<AppState>().api.post('/api/releases/grab', {
        ...widget.grabBody,
        'title': rel['title'],
        'guid': rel['guid'],
        'download_url': rel['download_url'],
        'info_hash': rel['info_hash'],
        'size_bytes': rel['size_bytes'],
        'seeders': rel['seeders'],
        'peers': rel['peers'],
        'indexer': rel['indexer'],
      });
      if (mounted) {
        snack(context, 'Sent ${rel['title']} to download client');
        await context.read<AppState>().refreshQueue();
        if (mounted) Navigator.of(context).pop();
      }
    } catch (e) {
      if (mounted) {
        setState(() => _grabbing = null);
        snack(context, '$e', error: true);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 16, 8, 8),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text('Search releases',
                        style: TextStyle(
                            fontSize: 18, fontWeight: FontWeight.w600)),
                    Text(widget.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                            fontSize: 13, color: Theme.of(context).hintColor)),
                  ],
                ),
              ),
              IconButton(
                onPressed: _search,
                icon: const Icon(Icons.refresh),
                tooltip: 'Search again',
              ),
              IconButton(
                onPressed: () => Navigator.pop(context),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
        ),
        const Divider(),
        Expanded(child: _body()),
      ],
    );
  }

  Widget _body() {
    if (_error != null) {
      return EmptyState(Icons.error_outline, 'Search failed',
          message: _error, action: OutlinedButton(
              onPressed: _search, child: const Text('Retry')));
    }
    if (_releases == null) {
      return const Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircularProgressIndicator(),
            SizedBox(height: 16),
            Text('Searching indexers...'),
          ],
        ),
      );
    }
    if (_releases!.isEmpty) {
      return EmptyState(Icons.search_off, 'No releases found',
          message: 'Add more indexers in Settings > Indexers');
    }
    return ListView.separated(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      itemCount: _releases!.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, i) => _ReleaseRow(
        rel: _releases![i],
        rank: i,
        grabbing: _grabbing == (_releases![i]['guid'] ?? _releases![i]['title']),
        onGrab: () => _grab(_releases![i]),
      ),
    );
  }
}

class _ReleaseRow extends StatelessWidget {
  final Map<String, dynamic> rel;
  final int rank;
  final bool grabbing;
  final VoidCallback onGrab;

  const _ReleaseRow({
    required this.rel,
    required this.rank,
    required this.grabbing,
    required this.onGrab,
  });

  @override
  Widget build(BuildContext context) {
    final q = rel['quality'] ?? {};
    final tags = <String>[
      if (q['resolution'] != null) '${q['resolution']}p',
      if (q['source'] != null) '${q['source']}',
      if (q['season_pack'] == true) 'PACK',
      if (q['proper'] == true) 'PROPER',
      if (q['repack'] == true) 'REPACK',
    ];
    final good = (rel['score'] ?? 0) > 0;
    return ListTile(
      contentPadding: const EdgeInsets.symmetric(vertical: 6),
      title: Row(
        children: [
          if (rank == 0 && good)
            const Padding(
              padding: EdgeInsets.only(right: 6),
              child: Icon(Icons.star, size: 14, color: F.warn),
            ),
          Expanded(
            child: Text(rel['title'] ?? '',
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                    fontSize: 13,
                    color: good ? null : Theme.of(context).hintColor)),
          ),
        ],
      ),
      subtitle: Padding(
        padding: const EdgeInsets.only(top: 6),
        child: Wrap(
          spacing: 8,
          runSpacing: 4,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            for (final t in tags) _Tag(t),
            _Tag(rel['indexer'] ?? ''),
            Text(
              '${fmtBytes(rel['size_bytes'] ?? 0)}  ·  '
              '${rel['seeders']}↑ ${rel['peers']}↓  ·  score ${rel['score']}',
              style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor),
            ),
          ],
        ),
      ),
      trailing: grabbing
          ? const SizedBox(
              width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
          : FilledButton.tonal(
              onPressed: onGrab,
              child: const Text('Grab'),
            ),
    );
  }
}

class _Tag extends StatelessWidget {
  final String text;
  const _Tag(this.text);

  @override
  Widget build(BuildContext context) {
    if (text.isEmpty) return const SizedBox.shrink();
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: F.accentSoft,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text,
          style: const TextStyle(
              fontSize: 10, fontWeight: FontWeight.w600, color: F.accent)),
    );
  }
}
