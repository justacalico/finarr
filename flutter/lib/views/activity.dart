import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// Queue + history in tabs.
class ActivityView extends StatelessWidget {
  const ActivityView({super.key});

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 2,
      child: Column(
        children: [
          const TabBar(tabs: [Tab(text: 'Queue'), Tab(text: 'History')]),
          const Expanded(
            child: TabBarView(children: [_QueueTab(), _HistoryTab()]),
          ),
        ],
      ),
    );
  }
}

class _QueueTab extends StatelessWidget {
  const _QueueTab();

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    if (s.queue.isEmpty) {
      return const EmptyState(Icons.inbox_outlined, 'Queue is empty');
    }
    return RefreshIndicator(
      onRefresh: s.refreshQueue,
      child: ListView.separated(
        padding: EdgeInsets.all(width >= 900 ? 32 : 16),
        itemCount: s.queue.length,
        separatorBuilder: (_, _) => const SizedBox(height: 8),
        itemBuilder: (context, i) => _QueueItem(item: s.queue[i]),
      ),
    );
  }
}

class _QueueItem extends StatelessWidget {
  final Map<String, dynamic> item;
  const _QueueItem({required this.item});

  @override
  Widget build(BuildContext context) {
    final live = item['live'];
    final speed = live != null ? live['download_speed'] ?? 0 : 0;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                StatusChip(item['state'] ?? ''),
                const SizedBox(width: 8),
                if (item['indexer'] != '')
                  Flexible(
                    child: Text(item['indexer'],
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                            fontSize: 11,
                            color: Theme.of(context).hintColor)),
                  )
                else
                  const Spacer(),
                Text(fmtBytes(item['size_bytes'] ?? 0),
                    style: TextStyle(
                        fontSize: 11, color: Theme.of(context).hintColor)),
              ],
            ),
            const SizedBox(height: 8),
            Text(item['name'] ?? '',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
            if (item['target_title'] != '') ...[
              const SizedBox(height: 2),
              Text('→ ${item['target_title']}',
                  style: const TextStyle(fontSize: 12, color: F.accent)),
            ],
            const SizedBox(height: 10),
            Row(
              children: [
                Expanded(
                  child: LinearProgressIndicator(
                      value: (item['progress'] ?? 0).toDouble()),
                ),
                const SizedBox(width: 12),
                Text(fmtPct((item['progress'] ?? 0).toDouble()),
                    style: const TextStyle(fontSize: 12)),
              ],
            ),
            if (live != null)
              Padding(
                padding: const EdgeInsets.only(top: 6),
                child: Text(
                  '↓ ${fmtSpeed(speed)}  ·  ETA ${fmtEta(live['eta_seconds'])}',
                  style: const TextStyle(fontSize: 11, color: F.info),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _HistoryTab extends StatelessWidget {
  const _HistoryTab();

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    if (s.history.isEmpty) {
      return const EmptyState(Icons.history, 'No history yet');
    }
    return RefreshIndicator(
      onRefresh: s.refreshHistory,
      child: ListView.separated(
        padding: EdgeInsets.all(width >= 900 ? 32 : 16),
        itemCount: s.history.length,
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, i) {
          final h = s.history[i];
          final type = h['event_type'] ?? '';
          final (icon, color) = switch (type) {
            'grabbed' => (Icons.file_download_outlined, F.info),
            'imported' => (Icons.check_circle_outline, F.ok),
            'import_failed' => (Icons.error_outline, F.bad),
            _ => (Icons.circle_outlined, Colors.grey),
          };
          return ListTile(
            leading: Icon(icon, color: color, size: 20),
            title: Text(h['title'] ?? '',
                maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text(
              '${type.replaceAll('_', ' ')}${h['media_type'] != '' ? ' · ${h['media_type']}' : ''}',
              style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor),
            ),
            trailing: Text(fmtDateTime(h['created_at']),
                style: TextStyle(
                    fontSize: 11, color: Theme.of(context).hintColor)),
          );
        },
      ),
    );
  }
}
