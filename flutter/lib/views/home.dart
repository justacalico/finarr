import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// Dashboard: library counts, live speeds, queue summary, next episodes.
class HomeView extends StatelessWidget {
  const HomeView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final wide = MediaQuery.sizeOf(context).width >= 900;
    final downloading =
        s.queue.where((q) => q['state'] == 'live' || ((q['live']?['download_speed']) ?? 0) > 0).length;
    final upcoming = s.calendarItems.take(8).toList();
    final recent = s.history.take(8).toList();

    return RefreshIndicator(
      onRefresh: s.refreshAll,
      child: ListView(
        padding: EdgeInsets.all(wide ? 32 : 16),
        children: [
          const SectionHeader('Overview'),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _StatCard(Icons.movie_outlined, 'Movies', '${s.movies.length}',
                  s.movies.where((m) => m['status'] == 'imported').length, 'in library'),
              _StatCard(Icons.tv_outlined, 'Series', '${s.seriesList.length}',
                  s.seriesList.length, 'shows'),
              _StatCard(Icons.album_outlined, 'Artists', '${s.artists.length}',
                  s.artists.length, 'artists'),
              _StatCard(Icons.download_outlined, 'Active', '$downloading',
                  s.torrents.length, 'torrents'),
            ],
          ),
          const SizedBox(height: 28),
          LayoutBuilder(builder: (context, c) {
            final left = Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SectionHeader('Queue',
                    trailing: null),
                _QueuePreview(),
              ],
            );
            final right = Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SectionHeader('Upcoming'),
                _Upcoming(items: upcoming),
                const SizedBox(height: 24),
                const SectionHeader('Recent activity'),
                _RecentActivity(items: recent),
              ],
            );
            if (wide) {
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(flex: 5, child: left),
                  const SizedBox(width: 24),
                  Expanded(flex: 4, child: right),
                ],
              );
            }
            return Column(children: [left, const SizedBox(height: 24), right]);
          }),
        ],
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;
  final int sub;
  final String subLabel;
  const _StatCard(this.icon, this.label, this.value, this.sub, this.subLabel);

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 168,
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(icon, size: 20, color: F.accent),
              const SizedBox(height: 12),
              Text(value,
                  style: const TextStyle(
                      fontSize: 26, fontWeight: FontWeight.w700)),
              const SizedBox(height: 2),
              Text('$sub $subLabel',
                  style: TextStyle(
                      fontSize: 12, color: Theme.of(context).hintColor)),
              Text(label,
                  style: TextStyle(
                      fontSize: 12, color: Theme.of(context).hintColor)),
            ],
          ),
        ),
      ),
    );
  }
}

class _QueuePreview extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final items = s.queue.take(6).toList();
    if (items.isEmpty) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('Nothing in the queue')),
        ),
      );
    }
    return Card(
      child: Column(
        children: [
          for (final it in items)
            ListTile(
              dense: false,
              leading: StatusChip(it['state'] ?? ''),
              title: Text(it['name'] ?? '',
                  maxLines: 1, overflow: TextOverflow.ellipsis),
              subtitle: it['target_title'] != ''
                  ? Text(it['target_title'],
                      style: TextStyle(
                          fontSize: 11, color: Theme.of(context).hintColor))
                  : null,
              trailing: SizedBox(
                width: 120,
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    Text(fmtPct((it['progress'] ?? 0).toDouble()),
                        style: const TextStyle(fontSize: 12)),
                    const SizedBox(height: 4),
                    LinearProgressIndicator(
                        value: (it['progress'] ?? 0).toDouble()),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _Upcoming extends StatelessWidget {
  final List<dynamic> items;
  const _Upcoming({required this.items});

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('Nothing scheduled')),
        ),
      );
    }
    return Card(
      child: Column(
        children: [
          for (final it in items)
            ListTile(
              leading: Poster(
                  url: it['poster_url'],
                  title: it['series_title'] ?? it['title'] ?? '',
                  width: 34),
              title: Text(
                it['kind'] == 'episode'
                    ? '${it['series_title']} S${it['season'].toString().padLeft(2, '0')}E${it['episode'].toString().padLeft(2, '0')}'
                    : it['title'] ?? '',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Text(it['title'] ?? '',
                  maxLines: 1, overflow: TextOverflow.ellipsis),
              trailing: Text(fmtDate(it['date']),
                  style: TextStyle(
                      fontSize: 12, color: Theme.of(context).hintColor)),
            ),
        ],
      ),
    );
  }
}

class _RecentActivity extends StatelessWidget {
  final List<dynamic> items;
  const _RecentActivity({required this.items});

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('No activity yet')),
        ),
      );
    }
    return Card(
      child: Column(
        children: [
          for (final it in items)
            ListTile(
              leading: Icon(_iconFor(it['event_type']),
                  size: 18, color: Theme.of(context).hintColor),
              title: Text(it['title'] ?? '',
                  maxLines: 1, overflow: TextOverflow.ellipsis),
              subtitle: Text(_labelFor(it['event_type']),
                  style: const TextStyle(fontSize: 11)),
              trailing: Text(fmtDateTime(it['created_at']),
                  style: TextStyle(
                      fontSize: 11, color: Theme.of(context).hintColor)),
              dense: true,
            ),
        ],
      ),
    );
  }

  IconData _iconFor(String? t) => switch (t) {
        'grabbed' => Icons.file_download_outlined,
        'imported' => Icons.check_circle_outline,
        'import_failed' => Icons.error_outline,
        _ => Icons.circle_outlined,
      };

  String _labelFor(String? t) => switch (t) {
        'grabbed' => 'Grabbed',
        'imported' => 'Imported to library',
        'import_failed' => 'Import failed',
        _ => t ?? '',
      };
}
