import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

class SeriesView extends StatelessWidget {
  const SeriesView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    final cols = (width / 150).floor().clamp(2, 10);
    final cardW = (width - (width >= 900 ? 64 : 32) - (cols - 1) * 16) / cols;
    final cardRatio = cardW / (cardW * 1.5 + 92);
    return Scaffold(
      body: RefreshIndicator(
        onRefresh: s.refreshLibraries,
        child: CustomScrollView(
          slivers: [
            SliverPadding(
              padding: EdgeInsets.all(width >= 900 ? 32 : 16),
              sliver: SliverToBoxAdapter(
                child: SectionHeader(
                  'Series',
                  subtitle: '${s.seriesList.length} in library',
                  trailing: FilledButton.icon(
                    onPressed: () => _addSeries(context),
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add Series'),
                  ),
                ),
              ),
            ),
            if (s.seriesList.isEmpty)
              SliverFillRemaining(
                child: EmptyState(
                  Icons.tv_outlined,
                  'No series yet',
                  message: 'Add a series and Finarr monitors new episodes automatically.',
                  action: FilledButton.icon(
                    onPressed: () => _addSeries(context),
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add Series'),
                  ),
                ),
              )
            else
              SliverPadding(
                padding: EdgeInsets.symmetric(horizontal: width >= 900 ? 32 : 16),
                sliver: SliverGrid(
                  gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                    crossAxisCount: cols,
                    mainAxisSpacing: 16,
                    crossAxisSpacing: 16,
                    childAspectRatio: cardRatio,
                  ),
                  delegate: SliverChildBuilderDelegate(
                    (context, i) => _SeriesCard(series: s.seriesList[i]),
                    childCount: s.seriesList.length,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _addSeries(BuildContext context) async {
    final s = context.read<AppState>();
    final picked = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => const _SeriesSearchDialog(),
    );
    if (picked == null || !context.mounted) return;
    try {
      await s.api.post('/api/series', {'tvmaze_id': picked['tvmaze_id']});
      await s.refreshLibraries();
      if (context.mounted) snack(context, 'Added ${picked['title']}');
    } catch (e) {
      if (context.mounted) snack(context, '$e', error: true);
    }
  }
}

class _SeriesCard extends StatelessWidget {
  final Map<String, dynamic> series;
  const _SeriesCard({required this.series});

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(F.radius),
      onTap: () => context.push('/series/${series['id']}'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Poster(url: series['poster_url'], title: series['title'] ?? ''),
          const SizedBox(height: 8),
          Text(series['title'] ?? '',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style:
                  const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
          const SizedBox(height: 4),
          Row(
            children: [
              Flexible(
                child: Text(
                  '${series['year'] ?? ''}${series['network'] != null ? ' · ${series['network']}' : ''}'
                      .trim(),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                      fontSize: 11, color: Theme.of(context).hintColor),
                ),
              ),
              const SizedBox(width: 4),
              StatusChip(series['series_status'] == 'continuing'
                  ? 'continuing'
                  : series['series_status'] ?? ''),
            ],
          ),
        ],
      ),
    );
  }
}

class _SeriesSearchDialog extends StatefulWidget {
  const _SeriesSearchDialog();

  @override
  State<_SeriesSearchDialog> createState() => _SeriesSearchDialogState();
}

class _SeriesSearchDialogState extends State<_SeriesSearchDialog> {
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
          .get('/api/lookup/series', {'q': _q.text.trim()});
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
      title: const Text('Add series'),
      content: SizedBox(
        width: 480,
        height: 420,
        child: Column(
          children: [
            TextField(
              controller: _q,
              autofocus: true,
              decoration: InputDecoration(
                hintText: 'Search TVmaze...',
                suffixIcon: IconButton(
                    onPressed: _search, icon: const Icon(Icons.search)),
              ),
              onSubmitted: (_) => _search(),
            ),
            const SizedBox(height: 12),
            Expanded(child: _resultsBody()),
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

  Widget _resultsBody() {
    if (_busy) return const Center(child: CircularProgressIndicator());
    if (_error != null) {
      return Center(
          child: Text(_error!, style: const TextStyle(color: F.bad, fontSize: 13)));
    }
    if (_results == null) {
      return Center(
          child: Text('Search for a series to add',
              style: TextStyle(color: Theme.of(context).hintColor)));
    }
    if (_results!.isEmpty) {
      return const Center(child: Text('No results'));
    }
    return ListView.builder(
      itemCount: _results!.length,
      itemBuilder: (context, i) {
        final s = _results![i];
        return ListTile(
          leading: Poster(url: s['poster_url'], title: s['title'] ?? '', width: 34),
          title: Text(s['title'] ?? '', maxLines: 1, overflow: TextOverflow.ellipsis),
          subtitle: Text(
            '${s['year'] ?? ''} ${s['network'] ?? ''} ${s['status'] ?? ''}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(fontSize: 11),
          ),
          onTap: () => Navigator.pop(context, s),
        );
      },
    );
  }
}
