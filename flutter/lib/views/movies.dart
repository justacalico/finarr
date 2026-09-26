import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// Poster grid + "add movie" search flow.
class MoviesView extends StatelessWidget {
  const MoviesView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    final cols = (width / 150).floor().clamp(2, 10);
    return Scaffold(
      body: RefreshIndicator(
        onRefresh: s.refreshLibraries,
        child: CustomScrollView(
          slivers: [
            SliverPadding(
              padding: EdgeInsets.all(width >= 900 ? 32 : 16),
              sliver: SliverToBoxAdapter(
                child: SectionHeader(
                  'Movies',
                  subtitle: '${s.movies.length} in library',
                  trailing: FilledButton.icon(
                    onPressed: () => _addMovie(context),
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add Movie'),
                  ),
                ),
              ),
            ),
            if (s.movies.isEmpty)
              SliverFillRemaining(
                child: EmptyState(
                  Icons.movie_outlined,
                  'No movies yet',
                  message: 'Add a movie and Finarr will find, download and import it.',
                  action: FilledButton.icon(
                    onPressed: () => _addMovie(context),
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add Movie'),
                  ),
                ),
              )
            else
              SliverPadding(
                padding: EdgeInsets.symmetric(
                    horizontal: width >= 900 ? 32 : 16),
                sliver: SliverGrid(
                  gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                    crossAxisCount: cols,
                    mainAxisSpacing: 16,
                    crossAxisSpacing: 16,
                    childAspectRatio: 0.58,
                  ),
                  delegate: SliverChildBuilderDelegate(
                    (context, i) => _MovieCard(movie: s.movies[i]),
                    childCount: s.movies.length,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _addMovie(BuildContext context) async {
    final s = context.read<AppState>();
    final picked = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => const _MovieSearchDialog(),
    );
    if (picked == null || !context.mounted) return;
    try {
      await s.api.post('/api/movies', {'tmdb_id': picked['tmdb_id']});
      await s.refreshLibraries();
      if (context.mounted) snack(context, 'Added ${picked['title']}');
    } catch (e) {
      if (context.mounted) snack(context, '$e', error: true);
    }
  }
}

class _MovieCard extends StatelessWidget {
  final Map<String, dynamic> movie;
  const _MovieCard({required this.movie});

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(F.radius),
      onTap: () => context.push('/movie/${movie['id']}'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Stack(
            children: [
              Poster(url: movie['poster_url'], title: movie['title'] ?? ''),
              if (movie['status'] == 'downloading')
                Positioned(
                  left: 0,
                  right: 0,
                  bottom: 0,
                  child: Container(
                    height: 3,
                    color: F.info,
                  ),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Text(movie['title'] ?? '',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style:
                  const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
          const SizedBox(height: 4),
          Row(
            children: [
              if (movie['year'] != null)
                Text('${movie['year']}',
                    style: TextStyle(
                        fontSize: 11, color: Theme.of(context).hintColor)),
              const Spacer(),
              StatusChip(movie['status'] ?? ''),
            ],
          ),
        ],
      ),
    );
  }
}

class _MovieSearchDialog extends StatefulWidget {
  const _MovieSearchDialog();

  @override
  State<_MovieSearchDialog> createState() => _MovieSearchDialogState();
}

class _MovieSearchDialogState extends State<_MovieSearchDialog> {
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
          .get('/api/lookup/movies', {'q': _q.text.trim()});
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
      title: const Text('Add movie'),
      content: SizedBox(
        width: 480,
        height: 420,
        child: Column(
          children: [
            TextField(
              controller: _q,
              autofocus: true,
              decoration: InputDecoration(
                hintText: 'Search TMDB...',
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
          child: Text('Search for a movie to add',
              style: TextStyle(color: Theme.of(context).hintColor)));
    }
    if (_results!.isEmpty) {
      return const Center(child: Text('No results'));
    }
    return ListView.builder(
      itemCount: _results!.length,
      itemBuilder: (context, i) {
        final m = _results![i];
        return ListTile(
          leading: Poster(url: m['poster_url'], title: m['title'] ?? '', width: 34),
          title: Text(m['title'] ?? '', maxLines: 1, overflow: TextOverflow.ellipsis),
          subtitle: Text(
            '${m['year'] ?? ''}  ${m['overview'] ?? ''}',
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(fontSize: 11),
          ),
          onTap: () => Navigator.pop(context, m),
        );
      },
    );
  }
}
