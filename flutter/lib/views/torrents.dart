import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// qBittorrent-style torrent manager: toolbar, sortable list, detail
/// pane with per-file selection and rate limits.
class TorrentsView extends StatefulWidget {
  const TorrentsView({super.key});

  @override
  State<TorrentsView> createState() => _TorrentsViewState();
}

class _TorrentsViewState extends State<TorrentsView> {
  String? _selected;
  String _filter = 'all';

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final wide = MediaQuery.sizeOf(context).width >= 900;
    final items = _filtered(s.torrents);
    final sel = _selected == null
        ? null
        : items.cast<Map<String, dynamic>?>().firstWhere(
              (t) => t?['hash'] == _selected,
              orElse: () => null,
            );

    return Scaffold(
      body: Column(
        children: [
          _Toolbar(
            filter: _filter,
            onFilter: (f) => setState(() => _filter = f),
            onAddMagnet: _addMagnet,
            onAddFile: _addFile,
            onLimits: _editLimits,
          ),
          const Divider(height: 1),
          Expanded(
            child: wide
                ? Row(
                    children: [
                      Expanded(
                          flex: 3,
                          child: _TorrentList(
                              items: items,
                              selected: _selected,
                              onSelect: (h) => setState(() => _selected = h))),
                      const VerticalDivider(width: 1),
                      SizedBox(
                          width: 380,
                          child: _DetailPane(torrent: sel)),
                    ],
                  )
                : Column(
                    children: [
                      Expanded(
                        child: _TorrentList(
                            items: items,
                            selected: _selected,
                            onSelect: (h) => setState(() =>
                                _selected == h ? _selected = null : _selected = h)),
                      ),
                      if (sel != null)
                        Expanded(child: _DetailPane(torrent: sel)),
                    ],
                  ),
          ),
        ],
      ),
    );
  }

  List<dynamic> _filtered(List<dynamic> torrents) {
    return switch (_filter) {
      'downloading' => torrents
          .where((t) =>
              t['state'] == 'live' && !t['finished'])
          .toList(),
      'seeding' => torrents.where((t) => t['finished'] && t['state'] == 'live').toList(),
      'completed' => torrents.where((t) => t['finished']).toList(),
      'paused' => torrents.where((t) => t['state'] == 'paused').toList(),
      _ => torrents,
    };
  }

  Future<void> _addMagnet() async {
    final ctrl = TextEditingController();
    final cat = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        title: const Text('Add torrent'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: ctrl,
              autofocus: true,
              maxLines: 3,
              decoration: const InputDecoration(
                  hintText: 'magnet:?xt=urn:btih:...',
                  labelText: 'Magnet link or URL'),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: cat,
              decoration: const InputDecoration(
                  labelText: 'Category (optional)',
                  hintText: 'movies / series / music'),
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(c, false),
              child: const Text('Cancel')),
          FilledButton(
              onPressed: () => Navigator.pop(c, true), child: const Text('Add')),
        ],
      ),
    );
    if (ok != true || ctrl.text.trim().isEmpty || !mounted) return;
    try {
      await context.read<AppState>().api.post('/api/torrents', {
        'url': ctrl.text.trim(),
        if (cat.text.trim().isNotEmpty) 'category': cat.text.trim(),
      });
      if (mounted) snack(context, 'Torrent added');
    } catch (e) {
      if (mounted) snack(context, '$e', error: true);
    }
  }

  Future<void> _addFile() async {
    final files = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['torrent'],
    );
    final f = files.isEmpty ? null : files.first;
    if (f == null || !mounted) return;
    try {
      final bytes = await f.readAsBytes();
      if (!mounted) return;
      await context
          .read<AppState>()
          .api
          .addTorrentFile(bytes, f.name);
      if (mounted) snack(context, 'Torrent added');
    } catch (e) {
      if (mounted) snack(context, '$e', error: true);
    }
  }

  Future<void> _editLimits() async {
    final s = context.read<AppState>();
    final engine = s.settings['engine'] ?? {};
    final dl = TextEditingController(text: '${engine['download_kbps'] ?? 0}');
    final ul = TextEditingController(text: '${engine['upload_kbps'] ?? 0}');
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        title: const Text('Rate limits'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: dl,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Download (KiB/s, 0 = unlimited)'),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: ul,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Upload (KiB/s, 0 = unlimited)'),
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(c, false),
              child: const Text('Cancel')),
          FilledButton(
              onPressed: () => Navigator.pop(c, true), child: const Text('Apply')),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    await s.api.put('/api/torrents/limits', {
      'download_kbps': int.tryParse(dl.text) ?? 0,
      'upload_kbps': int.tryParse(ul.text) ?? 0,
    });
    if (mounted) snack(context, 'Rate limits applied');
  }
}

class _Toolbar extends StatelessWidget {
  final String filter;
  final ValueChanged<String> onFilter;
  final VoidCallback onAddMagnet;
  final VoidCallback onAddFile;
  final VoidCallback onLimits;

  const _Toolbar({
    required this.filter,
    required this.onFilter,
    required this.onAddMagnet,
    required this.onAddFile,
    required this.onLimits,
  });

  @override
  Widget build(BuildContext context) {
    final filters = ['all', 'downloading', 'seeding', 'completed', 'paused'];
    final chips = Wrap(
      spacing: 6,
      children: [
        for (final f in filters)
          ChoiceChip(
            label: Text(f),
            selected: filter == f,
            onSelected: (_) => onFilter(f),
            visualDensity: VisualDensity.compact,
          ),
      ],
    );
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              IconButton.filledTonal(
                tooltip: 'Add magnet / URL',
                onPressed: onAddMagnet,
                icon: const Icon(Icons.add_link),
              ),
              const SizedBox(width: 4),
              IconButton.filledTonal(
                tooltip: 'Add .torrent file',
                onPressed: onAddFile,
                icon: const Icon(Icons.upload_file_outlined),
              ),
              const SizedBox(width: 4),
              IconButton.filledTonal(
                tooltip: 'Rate limits',
                onPressed: onLimits,
                icon: const Icon(Icons.speed_outlined),
              ),
            ],
          ),
          const SizedBox(height: 8),
          chips,
        ],
      ),
    );
  }
}

class _TorrentList extends StatelessWidget {
  final List<dynamic> items;
  final String? selected;
  final ValueChanged<String> onSelect;

  const _TorrentList({
    required this.items,
    required this.selected,
    required this.onSelect,
  });

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const EmptyState(
          Icons.download_outlined, 'No torrents',
          message: 'Add a magnet link or .torrent file to get started.');
    }
    return ListView.separated(
      itemCount: items.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, i) {
        final t = items[i];
        final progress = (t['progress'] ?? 0).toDouble();
        final isSel = t['hash'] == selected;
        return InkWell(
          onTap: () => onSelect(t['hash']),
          child: Container(
            color: isSel ? F.accentSoft : null,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(t['name'] ?? '',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                              fontSize: 13, fontWeight: FontWeight.w600)),
                    ),
                    const SizedBox(width: 8),
                    StatusChip(t['state'] ?? ''),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      flex: 3,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          LinearProgressIndicator(value: progress),
                          const SizedBox(height: 4),
                          Text(
                            '${fmtPct(progress)} of ${fmtBytes(t['total_bytes'] ?? 0)}',
                            style: TextStyle(
                                fontSize: 11,
                                color: Theme.of(context).hintColor),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 16),
                    _SpeedCol(
                      down: t['download_speed'] ?? 0,
                      up: t['upload_speed'] ?? 0,
                    ),
                    Text(
                      'ETA ${fmtEta(t['eta_seconds'])}',
                      style: TextStyle(
                          fontSize: 11, color: Theme.of(context).hintColor),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _SpeedCol extends StatelessWidget {
  final int down;
  final int up;
  const _SpeedCol({required this.down, required this.up});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(right: 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Text('↓ ${fmtSpeed(down)}',
              style: const TextStyle(fontSize: 11, color: F.info)),
          Text('↑ ${fmtSpeed(up)}',
              style: const TextStyle(fontSize: 11, color: F.ok)),
        ],
      ),
    );
  }
}

class _DetailPane extends StatefulWidget {
  final Map<String, dynamic>? torrent;
  const _DetailPane({required this.torrent});

  @override
  State<_DetailPane> createState() => _DetailPaneState();
}

class _DetailPaneState extends State<_DetailPane> {
  List<dynamic>? _files;

  @override
  void initState() {
    super.initState();
    _loadFiles();
  }

  @override
  void didUpdateWidget(covariant _DetailPane old) {
    super.didUpdateWidget(old);
    if (old.torrent?['hash'] != widget.torrent?['hash']) {
      _files = null;
      _loadFiles();
    }
  }

  Future<void> _loadFiles() async {
    final t = widget.torrent;
    if (t == null) return;
    try {
      final r = await context
          .read<AppState>()
          .api
          .get('/api/torrents/${t['hash']}/files');
      if (mounted) setState(() => _files = r['files'] ?? []);
    } catch (_) {
      if (mounted) setState(() => _files = []);
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = widget.torrent;
    if (t == null) {
      return const EmptyState(Icons.info_outline, 'Select a torrent');
    }
    final paused = t['state'] == 'paused';
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(t['name'] ?? '',
            style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600)),
        const SizedBox(height: 4),
        Text(t['hash'] ?? '',
            style: TextStyle(
                fontFamily: 'monospace',
                fontSize: 10,
                color: Theme.of(context).hintColor)),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(
              onPressed: () => _act(paused ? 'resume' : 'pause'),
              icon: Icon(paused ? Icons.play_arrow : Icons.pause, size: 18),
              label: Text(paused ? 'Resume' : 'Pause'),
            ),
            OutlinedButton.icon(
              onPressed: _delete,
              icon: const Icon(Icons.delete_outline, size: 18),
              label: const Text('Delete'),
            ),
          ],
        ),
        const SizedBox(height: 16),
        CardSection(
          title: 'Details',
          children: [
            Wrap(
              spacing: 24,
              runSpacing: 12,
              children: [
                Stat('Size', fmtBytes(t['total_bytes'] ?? 0)),
                Stat('Downloaded', fmtBytes(t['progress_bytes'] ?? 0)),
                Stat('Uploaded', fmtBytes(t['uploaded_bytes'] ?? 0)),
                Stat('Peers',
                    '${t['peers']?['live'] ?? 0} live / ${t['peers']?['seen'] ?? 0} seen'),
                Stat('Ratio', _ratio(t)),
              ],
            ),
            const SizedBox(height: 8),
            Text('Save path: ${t['save_path']}',
                style: TextStyle(
                    fontSize: 11, color: Theme.of(context).hintColor)),
          ],
        ),
        const SizedBox(height: 16),
        CardSection(
          title: 'Files',
          children: [
            if (_files == null)
              const Center(
                  child: Padding(
                      padding: EdgeInsets.all(16),
                      child: CircularProgressIndicator()))
            else if (_files!.isEmpty)
              Text('Metadata not resolved yet',
                  style: TextStyle(color: Theme.of(context).hintColor))
            else
              for (final f in _files!) _fileRow(f),
          ],
        ),
      ],
    );
  }

  String _ratio(Map<String, dynamic> t) {
    final total = (t['total_bytes'] ?? 0);
    if (total == 0) return '-';
    return ((t['uploaded_bytes'] ?? 0) / total).toStringAsFixed(2);
  }

  Widget _fileRow(Map<String, dynamic> f) {
    final done = (f['size'] ?? 0) > 0 ? (f['downloaded'] ?? 0) / (f['size'] ?? 1) : 0.0;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        children: [
          SizedBox(
            width: 24,
            child: Checkbox(
              value: f['included'] == true,
              onChanged: (v) async {
                final current = _files!
                    .where((x) => x['included'] == true)
                    .map((x) => x['index'] as int)
                    .toSet();
                if (v == true) {
                  current.add(f['index']);
                } else {
                  current.remove(f['index']);
                }
                await context.read<AppState>().api.put(
                    '/api/torrents/${widget.torrent!['hash']}/only-files',
                    {'files': current.toList()});
                _loadFiles();
              },
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(f['path'] ?? '',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 12)),
                const SizedBox(height: 3),
                LinearProgressIndicator(value: done.toDouble()),
              ],
            ),
          ),
          const SizedBox(width: 10),
          Text(fmtBytes(f['size'] ?? 0),
              style: TextStyle(
                  fontSize: 11, color: Theme.of(context).hintColor)),
        ],
      ),
    );
  }

  Future<void> _act(String action) async {
    final t = widget.torrent!;
    final s = context.read<AppState>();
    try {
      await s.api.post('/api/torrents/${t['hash']}/$action');
    } catch (e) {
      if (mounted) snack(context, '$e', error: true);
    }
    await s.refreshTorrents();
  }

  Future<void> _delete() async {
    final t = widget.torrent!;
    final s = context.read<AppState>();
    final choice = await confirmDelete(context, title: 'Delete torrent');
    if (!mounted || choice == null) return;
    try {
      await s.api.delete('/api/torrents/${t['hash']}',
          choice == 'files' ? {'delete_files': 'true'} : null);
    } catch (e) {
      if (mounted) snack(context, '$e', error: true);
      return;
    }
    await s.refreshTorrents();
  }
}
