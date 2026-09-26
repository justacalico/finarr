import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';

/// Settings: every section stored server-side, editable in place.
class SettingsView extends StatefulWidget {
  const SettingsView({super.key});

  @override
  State<SettingsView> createState() => _SettingsViewState();
}

class _SettingsViewState extends State<SettingsView> {
  int _section = 0;

  static const _sections = [
    ('General', Icons.tune),
    ('Media Paths', Icons.folder_outlined),
    ('Indexers', Icons.hub_outlined),
    ('Download Clients', Icons.download_for_offline_outlined),
    ('Engine', Icons.electric_bolt_outlined),
    ('Metadata', Icons.info_outline),
    ('Automation', Icons.schedule_outlined),
    ('Notifications', Icons.notifications_outlined),
    ('Users', Icons.people_outline),
    ('System', Icons.dns_outlined),
  ];

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final wide = MediaQuery.sizeOf(context).width >= 900;
    return Row(
      children: [
        SizedBox(
          width: wide ? 230 : 64,
          child: ListView(
            padding: const EdgeInsets.symmetric(vertical: 16),
            children: [
              for (var i = 0; i < _sections.length; i++)
                _NavTile(
                  icon: _sections[i].$2,
                  label: _sections[i].$1,
                  selected: _section == i,
                  compact: !wide,
                  onTap: () => setState(() => _section = i),
                ),
            ],
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: ListView(
            padding: EdgeInsets.all(wide ? 32 : 16),
            children: [
              switch (_section) {
                0 => _GeneralSection(s: s),
                1 => _PathsSection(s: s),
                2 => _IndexersSection(s: s),
                3 => _ClientsSection(s: s),
                4 => _EngineSection(s: s),
                5 => _MetadataSection(s: s),
                6 => _AutomationSection(s: s),
                7 => _NotificationsSection(s: s),
                8 => _UsersSection(s: s),
                _ => _SystemSection(s: s),
              },
            ],
          ),
        ),
      ],
    );
  }
}

class _NavTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final bool compact;
  final VoidCallback onTap;
  const _NavTile(
      {required this.icon,
      required this.label,
      required this.selected,
      required this.compact,
      required this.onTap});

  @override
  Widget build(BuildContext context) {
    final child = InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: selected ? F.accentSoft : null,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          children: [
            Icon(icon,
                size: 18, color: selected ? F.accent : Theme.of(context).hintColor),
            if (!compact) ...[
              const SizedBox(width: 10),
              Expanded(
                child: Text(label,
                    style: TextStyle(
                        fontSize: 13,
                        fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                        color: selected ? F.accent : null)),
              ),
            ],
          ],
        ),
      ),
    );
    return compact ? Tooltip(message: label, child: child) : child;
  }
}

/// Shared section scaffolding: typed JSON settings behind form fields.
mixin _SectionState<T extends StatefulWidget> on State<T> {
  AppState get s;
  String get section;
  bool saving = false;
  Map<String, dynamic> get current => s.settings[section] ?? {};

  Future<void> save(Map<String, dynamic> body) async {
    setState(() => saving = true);
    try {
      await s.api.put('/api/settings/$section', body);
      await s.refreshSettings();
      if (mounted) snack(context, 'Saved');
    } catch (e) {
      if (mounted) snack(context, '$e', error: true);
    } finally {
      if (mounted) setState(() => saving = false);
    }
  }
}

TextEditingController _c(Object? v) =>
    TextEditingController(text: v == null ? '' : '$v');

class _GeneralSection extends StatefulWidget {
  final AppState s;
  const _GeneralSection({required this.s});
  @override
  State<_GeneralSection> createState() => _GeneralSectionState();
}

class _GeneralSectionState extends State<_GeneralSection>
    with _SectionState {
  @override
  AppState get s => widget.s;
  @override
  String get section => 'general';
  late final _name = _c(current['instance_name']);
  late final _host = _c(current['host']);
  late final _port = _c(current['port']);

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('General'),
        CardSection(children: [
          TextField(
              controller: _name,
              decoration: const InputDecoration(labelText: 'Instance name')),
          const SizedBox(height: 12),
          TextField(
              controller: _host,
              decoration: const InputDecoration(
                  labelText: 'Bind address', hintText: '0.0.0.0')),
          const SizedBox(height: 12),
          TextField(
              controller: _port,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Port',
                  helperText: 'Restart required to apply')),
          const SizedBox(height: 16),
          FilledButton(
              onPressed: saving
                  ? null
                  : () => save({
                        'instance_name': _name.text,
                        'host': _host.text,
                        'port': int.tryParse(_port.text) ?? 8787,
                        'allow_server_override':
                            current['allow_server_override'] ?? false,
                      }),
              child: const Text('Save')),
        ]),
        const SizedBox(height: 20),
        CardSection(title: 'Appearance', children: [
          SegmentedButton<String>(
            segments: const [
              ButtonSegment(value: 'system', label: Text('System')),
              ButtonSegment(value: 'light', label: Text('Light')),
              ButtonSegment(value: 'dark', label: Text('Dark')),
            ],
            selected: {s.themeMode},
            onSelectionChanged: (v) => s.setThemeMode(v.first),
          ),
        ]),
      ],
    );
  }
}

class _PathsSection extends StatefulWidget {
  final AppState s;
  const _PathsSection({required this.s});
  @override
  State<_PathsSection> createState() => _PathsSectionState();
}

class _PathsSectionState extends State<_PathsSection> with _SectionState {
  @override
  AppState get s => widget.s;
  @override
  String get section => 'paths';
  late final _downloads = _c(current['downloads_dir']);
  late final _movies = _c(current['movies_root']);
  late final _series = _c(current['series_root']);
  late final _music = _c(current['music_root']);
  late String _importMode = current['import_mode'] ?? 'hardlink';

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('Media Paths'),
        CardSection(children: [
          TextField(
              controller: _downloads,
              decoration: const InputDecoration(
                  labelText: 'Downloads folder',
                  helperText: 'Engine writes torrent data here')),
          const SizedBox(height: 12),
          TextField(
              controller: _movies,
              decoration: const InputDecoration(labelText: 'Movies library')),
          const SizedBox(height: 12),
          TextField(
              controller: _series,
              decoration: const InputDecoration(labelText: 'Series library')),
          const SizedBox(height: 12),
          TextField(
              controller: _music,
              decoration: const InputDecoration(labelText: 'Music library')),
          const SizedBox(height: 12),
          DropdownButtonFormField<String>(
            initialValue: _importMode,
            decoration: const InputDecoration(
                labelText: 'Import mode',
                helperText: 'Hardlink keeps seeding without using extra space'),
            items: const [
              DropdownMenuItem(value: 'hardlink', child: Text('Hardlink')),
              DropdownMenuItem(value: 'copy', child: Text('Copy')),
              DropdownMenuItem(value: 'move', child: Text('Move')),
            ],
            onChanged: (v) => setState(() => _importMode = v ?? 'hardlink'),
          ),
          const SizedBox(height: 16),
          FilledButton(
              onPressed: saving
                  ? null
                  : () => save({
                        'downloads_dir': _downloads.text,
                        'movies_root': _movies.text,
                        'series_root': _series.text,
                        'music_root': _music.text,
                        'import_mode': _importMode,
                      }),
              child: const Text('Save')),
        ]),
      ],
    );
  }
}

class _IndexersSection extends StatefulWidget {
  final AppState s;
  const _IndexersSection({required this.s});
  @override
  State<_IndexersSection> createState() => _IndexersSectionState();
}

class _IndexersSectionState extends State<_IndexersSection> {
  @override
  Widget build(BuildContext context) {
    final s = widget.s;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeader('Indexers',
            subtitle:
                'Torznab endpoints — Jackett, Prowlarr, or any compatible tracker',
            trailing: FilledButton.icon(
                onPressed: () => _edit(context, null),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('Add'))),
        if (s.indexers.isEmpty)
          const EmptyState(Icons.hub_outlined, 'No indexers',
              message:
                  'Add your Jackett endpoint:\nhttp://host:9117/api/v2.0/indexers/all/results/torznab/')
        else
          Card(
            child: Column(children: [
              for (final idx in s.indexers) _IndexerTile(indexer: idx, onEdit: () => _edit(context, idx)),
            ]),
          ),
      ],
    );
  }

  Future<void> _edit(BuildContext context, Map<String, dynamic>? existing) async {
    final s = context.read<AppState>();
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => _IndexerDialog(existing: existing),
    );
    if (saved == true) s.refreshIndexers();
  }
}

class _IndexerTile extends StatelessWidget {
  final Map<String, dynamic> indexer;
  final VoidCallback onEdit;
  const _IndexerTile({required this.indexer, required this.onEdit});

  @override
  Widget build(BuildContext context) {
    final s = context.read<AppState>();
    return ListTile(
      leading: Icon(Icons.hub,
          color: indexer['enabled'] == 1 ? F.accent : Theme.of(context).hintColor),
      title: Text(indexer['name'] ?? ''),
      subtitle: Text(indexer['url'] ?? '',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(fontSize: 11)),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            tooltip: 'Test',
            icon: const Icon(Icons.wifi_tethering, size: 18),
            onPressed: () async {
              final r = await s.api.post('/api/indexers/${indexer['id']}/test');
              if (context.mounted) {
                snack(
                    context,
                    r['ok'] == true
                        ? 'Indexer OK'
                        : 'Failed: ${r['error'] ?? 'unknown'}',
                    error: r['ok'] != true);
              }
            },
          ),
          IconButton(
            tooltip: 'Edit',
            icon: const Icon(Icons.edit_outlined, size: 18),
            onPressed: onEdit,
          ),
          IconButton(
            tooltip: 'Delete',
            icon: const Icon(Icons.delete_outline, size: 18),
            onPressed: () async {
              final ok = await confirm(context,
                  title: 'Delete indexer?',
                  message: indexer['name'] ?? '',
                  confirmLabel: 'Delete',
                  destructive: true);
              if (ok) {
                await s.api.delete('/api/indexers/${indexer['id']}');
                s.refreshIndexers();
              }
            },
          ),
        ],
      ),
    );
  }
}

class _IndexerDialog extends StatefulWidget {
  final Map<String, dynamic>? existing;
  const _IndexerDialog({required this.existing});

  @override
  State<_IndexerDialog> createState() => _IndexerDialogState();
}

class _IndexerDialogState extends State<_IndexerDialog> {
  late final _name =
      TextEditingController(text: widget.existing?['name'] ?? '');
  late final _url = TextEditingController(text: widget.existing?['url'] ?? '');
  late final _key =
      TextEditingController(text: widget.existing?['api_key'] ?? '');
  late bool _enabled = widget.existing?['enabled'] != 0;
  bool _testing = false;
  String? _testResult;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.existing == null ? 'Add indexer' : 'Edit indexer'),
      content: SizedBox(
        width: 440,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
                controller: _name,
                decoration: const InputDecoration(labelText: 'Name')),
            const SizedBox(height: 12),
            TextField(
                controller: _url,
                decoration: const InputDecoration(
                    labelText: 'Torznab URL',
                    hintText:
                        'http://host:9117/api/v2.0/indexers/all/results/torznab/')),
            const SizedBox(height: 12),
            TextField(
                controller: _key,
                decoration: const InputDecoration(labelText: 'API key')),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Enabled'),
              value: _enabled,
              onChanged: (v) => setState(() => _enabled = v),
            ),
            if (_testResult != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(_testResult!,
                    style: TextStyle(
                        fontSize: 12,
                        color: _testResult!.startsWith('OK')
                            ? F.ok
                            : F.bad)),
              ),
          ],
        ),
      ),
      actions: [
        TextButton.icon(
          onPressed: _testing
              ? null
              : () async {
                  setState(() {
                    _testing = true;
                    _testResult = null;
                  });
                  try {
                    final r = await context.read<AppState>().api.post(
                        '/api/indexers/test',
                        {'name': _name.text, 'url': _url.text, 'api_key': _key.text});
                    setState(() => _testResult = r['ok'] == true
                        ? 'OK — caps retrieved'
                        : 'Failed: ${r['error']}');
                  } catch (e) {
                    setState(() => _testResult = '$e');
                  } finally {
                    setState(() => _testing = false);
                  }
                },
          icon: const Icon(Icons.wifi_tethering, size: 16),
          label: const Text('Test'),
        ),
        TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel')),
        FilledButton(
          onPressed: _save,
          child: const Text('Save'),
        ),
      ],
    );
  }

  Future<void> _save() async {
    final s = context.read<AppState>();
    final body = {
      'name': _name.text.trim(),
      'url': _url.text.trim(),
      'api_key': _key.text.trim(),
      'enabled': _enabled,
    };
    try {
      if (widget.existing == null) {
        await s.api.post('/api/indexers', body);
      } else {
        await s.api.put('/api/indexers/${widget.existing!['id']}', body);
      }
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      setState(() => _testResult = '$e');
    }
  }
}

class _ClientsSection extends StatefulWidget {
  final AppState s;
  const _ClientsSection({required this.s});
  @override
  State<_ClientsSection> createState() => _ClientsSectionState();
}

class _ClientsSectionState extends State<_ClientsSection> {
  @override
  Widget build(BuildContext context) {
    final s = widget.s;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeader('Download Clients',
            subtitle: 'Built-in engine is always available; externals take priority',
            trailing: FilledButton.icon(
                onPressed: () => _edit(context, null),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('Add'))),
        Card(
          child: ListTile(
            leading: const Icon(Icons.bolt, color: F.accent),
            title: const Text('Built-in engine'),
            subtitle: Text('librqbit · always available',
                style: TextStyle(
                    fontSize: 11, color: Theme.of(context).hintColor)),
            trailing: const StatusChip('imported'),
          ),
        ),
        const SizedBox(height: 12),
        if (s.clients.isEmpty)
          const SizedBox.shrink()
        else
          Card(
            child: Column(children: [
              for (final c in s.clients) _ClientTile(client: c),
            ]),
          ),
      ],
    );
  }

  Future<void> _edit(BuildContext context, Map<String, dynamic>? existing) async {
    final s = context.read<AppState>();
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => _ClientDialog(existing: existing),
    );
    if (saved == true) s.refreshClients();
  }
}

class _ClientTile extends StatelessWidget {
  final Map<String, dynamic> client;
  const _ClientTile({required this.client});

  @override
  Widget build(BuildContext context) {
    final s = context.read<AppState>();
    final cfg = client['settings'] ?? {};
    return ListTile(
      leading: const Icon(Icons.download_for_offline_outlined),
      title: Text(client['name'] ?? ''),
      subtitle: Text('${client['impl']} · ${cfg['host'] ?? ''}',
          style: const TextStyle(fontSize: 11)),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            tooltip: 'Test',
            icon: const Icon(Icons.wifi_tethering, size: 18),
            onPressed: () async {
              final r = await s.api.post('/api/clients/${client['id']}/test');
              if (context.mounted) {
                snack(
                    context,
                    r['ok'] == true
                        ? 'Connected — ${r['torrents']} torrents'
                        : 'Failed: ${r['error']}',
                    error: r['ok'] != true);
              }
            },
          ),
          IconButton(
            tooltip: 'Edit',
            icon: const Icon(Icons.edit_outlined, size: 18),
            onPressed: () async {
              final saved = await showDialog<bool>(
                context: context,
                builder: (_) => _ClientDialog(existing: client),
              );
              if (saved == true) s.refreshClients();
            },
          ),
          IconButton(
            tooltip: 'Delete',
            icon: const Icon(Icons.delete_outline, size: 18),
            onPressed: () async {
              final ok = await confirm(context,
                  title: 'Delete client?',
                  message: client['name'] ?? '',
                  confirmLabel: 'Delete',
                  destructive: true);
              if (ok) {
                await s.api.delete('/api/clients/${client['id']}');
                s.refreshClients();
              }
            },
          ),
        ],
      ),
    );
  }
}

class _ClientDialog extends StatefulWidget {
  final Map<String, dynamic>? existing;
  const _ClientDialog({required this.existing});

  @override
  State<_ClientDialog> createState() => _ClientDialogState();
}

class _ClientDialogState extends State<_ClientDialog> {
  late final _name =
      TextEditingController(text: widget.existing?['name'] ?? '');
  late final _host = TextEditingController(
      text: widget.existing?['settings']?['host'] ?? '');
  late final _user = TextEditingController(
      text: widget.existing?['settings']?['username'] ?? 'admin');
  late final _pass = TextEditingController(
      text: widget.existing?['settings']?['password'] ?? '');
  late final _cat = TextEditingController(
      text: widget.existing?['settings']?['category'] ?? 'finarr');
  bool _testing = false;
  String? _testResult;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(
          widget.existing == null ? 'Add download client' : 'Edit client'),
      content: SizedBox(
        width: 440,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
                controller: _name,
                decoration: const InputDecoration(labelText: 'Name')),
            const SizedBox(height: 12),
            TextField(
                controller: _host,
                decoration: const InputDecoration(
                    labelText: 'qBittorrent URL',
                    hintText: 'http://192.168.1.10:8080')),
            const SizedBox(height: 12),
            TextField(
                controller: _user,
                decoration: const InputDecoration(labelText: 'Username')),
            const SizedBox(height: 12),
            TextField(
                controller: _pass,
                obscureText: true,
                decoration: const InputDecoration(labelText: 'Password')),
            const SizedBox(height: 12),
            TextField(
                controller: _cat,
                decoration: const InputDecoration(labelText: 'Category')),
            if (_testResult != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(_testResult!,
                    style: TextStyle(
                        fontSize: 12,
                        color: _testResult!.startsWith('OK') ? F.ok : F.bad)),
              ),
          ],
        ),
      ),
      actions: [
        TextButton.icon(
          onPressed: _testing
              ? null
              : () async {
                  setState(() {
                    _testing = true;
                    _testResult = null;
                  });
                  try {
                    final r = await context.read<AppState>().api.post(
                        '/api/clients/test',
                        _body());
                    setState(() => _testResult = r['ok'] == true
                        ? 'OK — connected'
                        : 'Failed: ${r['error']}');
                  } catch (e) {
                    setState(() => _testResult = '$e');
                  } finally {
                    setState(() => _testing = false);
                  }
                },
          icon: const Icon(Icons.wifi_tethering, size: 16),
          label: const Text('Test'),
        ),
        TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel')),
        FilledButton(onPressed: _save, child: const Text('Save')),
      ],
    );
  }

  Map<String, dynamic> _body() => {
        'name': _name.text.trim(),
        'host': _host.text.trim(),
        'username': _user.text.trim(),
        'password': _pass.text,
        'category': _cat.text.trim(),
      };

  Future<void> _save() async {
    final s = context.read<AppState>();
    try {
      if (widget.existing == null) {
        await s.api.post('/api/clients', _body());
      } else {
        await s.api.put('/api/clients/${widget.existing!['id']}', _body());
      }
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      setState(() => _testResult = '$e');
    }
  }
}

class _EngineSection extends StatefulWidget {
  final AppState s;
  const _EngineSection({required this.s});
  @override
  State<_EngineSection> createState() => _EngineSectionState();
}

class _EngineSectionState extends State<_EngineSection> with _SectionState {
  @override
  AppState get s => widget.s;
  @override
  String get section => 'engine';
  late final _port = _c(current['listen_port']);
  late final _dl = _c(current['download_kbps']);
  late final _ul = _c(current['upload_kbps']);
  late final _ratio = _c(current['seed_ratio']);
  late final _peers = _c(current['peer_limit']);
  late bool _dht = current['dht_enabled'] ?? true;
  late bool _lsd = current['lsd_enabled'] ?? true;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('Engine',
            subtitle: 'Built-in BitTorrent client (librqbit)'),
        CardSection(children: [
          TextField(
              controller: _port,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Listen port',
                  helperText: 'Incoming peer connections · engine restart applies it')),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('DHT'),
            subtitle: const Text('Trackerless peer discovery'),
            value: _dht,
            onChanged: (v) => setState(() => _dht = v),
          ),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Local peer discovery'),
            subtitle: const Text('Find peers on the LAN'),
            value: _lsd,
            onChanged: (v) => setState(() => _lsd = v),
          ),
          const SizedBox(height: 4),
          TextField(
              controller: _dl,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Download limit (KiB/s, 0 = unlimited)')),
          const SizedBox(height: 12),
          TextField(
              controller: _ul,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Upload limit (KiB/s, 0 = unlimited)')),
          const SizedBox(height: 12),
          TextField(
              controller: _ratio,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Seed ratio limit (0 = seed forever)')),
          const SizedBox(height: 12),
          TextField(
              controller: _peers,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                  labelText: 'Peer limit per torrent (blank = default)')),
          const SizedBox(height: 16),
          Row(children: [
            FilledButton(
                onPressed: saving
                    ? null
                    : () => save({
                          'listen_port': int.tryParse(_port.text) ?? 4242,
                          'dht_enabled': _dht,
                          'lsd_enabled': _lsd,
                          'peer_limit': int.tryParse(_peers.text),
                          'download_kbps': int.tryParse(_dl.text) ?? 0,
                          'upload_kbps': int.tryParse(_ul.text) ?? 0,
                          'seed_ratio': double.tryParse(_ratio.text) ?? 0,
                        }),
                child: const Text('Save')),
            const SizedBox(width: 12),
            OutlinedButton.icon(
              onPressed: () async {
                try {
                  await s.api.post('/api/settings/engine/restart');
                  if (context.mounted) snack(context, 'Engine restarting');
                } catch (e) {
                  if (context.mounted) snack(context, '$e', error: true);
                }
              },
              icon: const Icon(Icons.restart_alt, size: 18),
              label: const Text('Restart engine'),
            ),
          ]),
        ]),
      ],
    );
  }
}

class _MetadataSection extends StatefulWidget {
  final AppState s;
  const _MetadataSection({required this.s});
  @override
  State<_MetadataSection> createState() => _MetadataSectionState();
}

class _MetadataSectionState extends State<_MetadataSection>
    with _SectionState {
  @override
  AppState get s => widget.s;
  @override
  String get section => 'metadata';
  late final _tmdb = _c(current['tmdb_api_key']);

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('Metadata'),
        CardSection(children: [
          TextField(
              controller: _tmdb,
              obscureText: true,
              decoration: const InputDecoration(
                  labelText: 'TMDB API key',
                  helperText:
                      'Required for movie metadata. Series (TVmaze) and music (MusicBrainz) need no key.')),
          const SizedBox(height: 16),
          FilledButton(
              onPressed: saving
                  ? null
                  : () => save({'tmdb_api_key': _tmdb.text.trim()}),
              child: const Text('Save')),
        ]),
      ],
    );
  }
}

class _AutomationSection extends StatefulWidget {
  final AppState s;
  const _AutomationSection({required this.s});
  @override
  State<_AutomationSection> createState() => _AutomationSectionState();
}

class _AutomationSectionState extends State<_AutomationSection>
    with _SectionState {
  @override
  AppState get s => widget.s;
  @override
  String get section => 'automation';
  late final _interval = _c(current['search_interval_min']);
  late bool _searchEnabled = current['wanted_search_enabled'] ?? true;
  late bool _autoImport = current['auto_import'] ?? true;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('Automation'),
        CardSection(children: [
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Wanted search'),
            subtitle: const Text(
                'Periodically search indexers for missing monitored media'),
            value: _searchEnabled,
            onChanged: (v) => setState(() => _searchEnabled = v),
          ),
          TextField(
              controller: _interval,
              keyboardType: TextInputType.number,
              decoration:
                  const InputDecoration(labelText: 'Search interval (minutes)')),
          const SizedBox(height: 8),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Auto import'),
            subtitle: const Text(
                'Move finished downloads into the library automatically'),
            value: _autoImport,
            onChanged: (v) => setState(() => _autoImport = v),
          ),
          const SizedBox(height: 16),
          FilledButton(
              onPressed: saving
                  ? null
                  : () => save({
                        'search_interval_min':
                            int.tryParse(_interval.text) ?? 30,
                        'wanted_search_enabled': _searchEnabled,
                        'auto_import': _autoImport,
                      }),
              child: const Text('Save')),
        ]),
        const SizedBox(height: 20),
        CardSection(title: 'Library scan', children: [
          Text(
              'Scan library folders and link files that already exist on disk.',
              style: TextStyle(color: Theme.of(context).hintColor, fontSize: 13)),
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: () async {
              try {
                final r = await s.api.post('/api/scan');
                final rep = r['report'];
                if (context.mounted) {
                  snack(context,
                      'Scan done: ${rep['linked']} linked, ${rep['files_seen']} files seen');
                }
              } catch (e) {
                if (context.mounted) snack(context, '$e', error: true);
              }
            },
            icon: const Icon(Icons.find_in_page_outlined, size: 18),
            label: const Text('Scan library'),
          ),
        ]),
      ],
    );
  }
}

class _NotificationsSection extends StatefulWidget {
  final AppState s;
  const _NotificationsSection({required this.s});
  @override
  State<_NotificationsSection> createState() => _NotificationsSectionState();
}

class _NotificationsSectionState extends State<_NotificationsSection>
    with _SectionState {
  @override
  AppState get s => widget.s;
  @override
  String get section => 'notifications';
  late String _kind = current['kind'] ?? '';
  late final _url = _c(current['url']);
  late final _token = _c(current['token']);

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('Notifications'),
        CardSection(children: [
          DropdownButtonFormField<String>(
            initialValue: _kind.isEmpty ? null : _kind,
            decoration: const InputDecoration(labelText: 'Service'),
            hint: const Text('Disabled'),
            items: const [
              DropdownMenuItem(value: 'ntfy', child: Text('ntfy.sh')),
              DropdownMenuItem(value: 'discord', child: Text('Discord webhook')),
              DropdownMenuItem(value: 'gotify', child: Text('Gotify')),
              DropdownMenuItem(value: 'webhook', child: Text('Generic webhook')),
            ],
            onChanged: (v) => setState(() => _kind = v ?? ''),
          ),
          const SizedBox(height: 12),
          TextField(
              controller: _url,
              decoration: InputDecoration(
                  labelText: switch (_kind) {
                    'ntfy' => 'Topic URL',
                    'gotify' => 'Server URL',
                    _ => 'Webhook URL',
                  },
                  hintText: switch (_kind) {
                    'ntfy' => 'https://ntfy.sh/my-topic',
                    'gotify' => 'https://push.example.com',
                    _ => 'https://...',
                  })),
          if (_kind == 'gotify') ...[
            const SizedBox(height: 12),
            TextField(
                controller: _token,
                decoration: const InputDecoration(labelText: 'App token')),
          ],
          const SizedBox(height: 16),
          FilledButton(
              onPressed: saving
                  ? null
                  : () => save({
                        'kind': _kind,
                        'url': _url.text.trim(),
                        'token': _token.text.trim(),
                        'priority': current['priority'] ?? 3,
                        'events': current['events'] ?? ['import'],
                      }),
              child: const Text('Save')),
        ]),
      ],
    );
  }
}

class _UsersSection extends StatefulWidget {
  final AppState s;
  const _UsersSection({required this.s});
  @override
  State<_UsersSection> createState() => _UsersSectionState();
}

class _UsersSectionState extends State<_UsersSection> {
  @override
  void initState() {
    super.initState();
    widget.s.refreshUsers();
  }

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeader('Users',
            trailing: s.isAdmin
                ? FilledButton.icon(
                    onPressed: () => _addUser(context),
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add'))
                : null),
        if (!s.isAdmin)
          const EmptyState(Icons.lock_outline, 'Admins only')
        else
          Card(
            child: Column(children: [
              for (final u in s.users) _UserTile(user: u),
            ]),
          ),
        const SizedBox(height: 20),
        _PasswordCard(s: s),
      ],
    );
  }

  Future<void> _addUser(BuildContext context) async {
    final user = TextEditingController();
    final pass = TextEditingController();
    var admin = false;
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => StatefulBuilder(
        builder: (c, setS) => AlertDialog(
          title: const Text('Add user'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                  controller: user,
                  decoration: const InputDecoration(labelText: 'Username')),
              const SizedBox(height: 12),
              TextField(
                  controller: pass,
                  obscureText: true,
                  decoration: const InputDecoration(labelText: 'Password')),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Admin'),
                value: admin,
                onChanged: (v) => setS(() => admin = v),
              ),
            ],
          ),
          actions: [
            TextButton(
                onPressed: () => Navigator.pop(c, false),
                child: const Text('Cancel')),
            FilledButton(
                onPressed: () => Navigator.pop(c, true),
                child: const Text('Create')),
          ],
        ),
      ),
    );
    if (ok != true || !context.mounted) return;
    try {
      final s = context.read<AppState>();
      await s.api.post('/api/users', {
        'username': user.text.trim(),
        'password': pass.text,
        'role': admin ? 'admin' : 'user',
      });
      s.refreshUsers();
    } catch (e) {
      if (context.mounted) snack(context, '$e', error: true);
    }
  }
}

class _UserTile extends StatelessWidget {
  final Map<String, dynamic> user;
  const _UserTile({required this.user});

  @override
  Widget build(BuildContext context) {
    final s = context.read<AppState>();
    return ListTile(
      leading: CircleAvatar(
        radius: 14,
        backgroundColor: F.accentSoft,
        child: Text(
          (user['username'] ?? '?')[0].toUpperCase(),
          style: const TextStyle(color: F.accent, fontSize: 12),
        ),
      ),
      title: Text(user['username'] ?? ''),
      subtitle: Text(
          '${user['role']}${user['disabled'] == true ? ' · disabled' : ''}${user['last_login_at'] != null ? ' · last seen ${fmtDate(user['last_login_at'])}' : ''}',
          style: const TextStyle(fontSize: 11)),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            tooltip: 'Create API key',
            icon: const Icon(Icons.key_outlined, size: 18),
            onPressed: () async {
              final r = await s.api.post('/api/users/${user['id']}/apikey', {});
              if (context.mounted) {
                await showDialog(
                  context: context,
                  builder: (c) => AlertDialog(
                    title: const Text('API key'),
                    content: SelectableText(r['api_key'] ?? '',
                        style: const TextStyle(
                            fontFamily: 'monospace', fontSize: 12)),
                    actions: [
                      TextButton(
                          onPressed: () => Navigator.pop(c),
                          child: const Text('Close')),
                    ],
                  ),
                );
              }
            },
          ),
          IconButton(
            tooltip: 'Delete',
            icon: const Icon(Icons.delete_outline, size: 18),
            onPressed: () async {
              final ok = await confirm(context,
                  title: 'Delete user?',
                  message: user['username'] ?? '',
                  confirmLabel: 'Delete',
                  destructive: true);
              if (ok) {
                try {
                  await s.api.delete('/api/users/${user['id']}');
                  s.refreshUsers();
                } catch (e) {
                  if (context.mounted) snack(context, '$e', error: true);
                }
              }
            },
          ),
        ],
      ),
    );
  }
}

class _PasswordCard extends StatefulWidget {
  final AppState s;
  const _PasswordCard({required this.s});
  @override
  State<_PasswordCard> createState() => _PasswordCardState();
}

class _PasswordCardState extends State<_PasswordCard> {
  final _current = TextEditingController();
  final _next = TextEditingController();

  @override
  Widget build(BuildContext context) {
    return CardSection(
      title: 'Change your password',
      children: [
        TextField(
            controller: _current,
            obscureText: true,
            decoration: const InputDecoration(labelText: 'Current password')),
        const SizedBox(height: 12),
        TextField(
            controller: _next,
            obscureText: true,
            decoration: const InputDecoration(labelText: 'New password')),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: () async {
            try {
              await widget.s.api.post('/api/auth/password', {
                'current': _current.text,
                'new_password': _next.text,
              });
              if (context.mounted) {
                snack(context, 'Password changed');
                _current.clear();
                _next.clear();
              }
            } catch (e) {
              if (context.mounted) snack(context, '$e', error: true);
            }
          },
          child: const Text('Change password'),
        ),
      ],
    );
  }
}

class _SystemSection extends StatelessWidget {
  final AppState s;
  const _SystemSection({required this.s});

  @override
  Widget build(BuildContext context) {
    final info = s.systemInfo;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionHeader('System'),
        CardSection(title: 'Server', children: [
          Wrap(
            spacing: 32,
            runSpacing: 12,
            children: [
              Stat('Version', '${info['version'] ?? ''}'),
              Stat('SQLite', '${info['sqlite_version'] ?? ''}'),
              Stat('Uptime', _uptime(info['uptime_seconds'])),
              Stat('Listening', '${info['listen_addr'] ?? ''}'),
            ],
          ),
          const SizedBox(height: 8),
          Text('Data dir: ${info['data_dir'] ?? ''}',
              style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor)),
        ]),
        const SizedBox(height: 20),
        CardSection(title: 'Disk space', children: [
          if (s.disk.isEmpty)
            Text('No paths configured',
                style: TextStyle(color: Theme.of(context).hintColor)),
          for (final d in s.disk)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 6),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                          child: Text(d['path'],
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(fontSize: 13))),
                      Text(
                          '${fmtBytes((d['total_bytes'] ?? 0) - (d['free_bytes'] ?? 0))} / ${fmtBytes(d['total_bytes'] ?? 0)}',
                          style: TextStyle(
                              fontSize: 11, color: Theme.of(context).hintColor)),
                    ],
                  ),
                  const SizedBox(height: 4),
                  LinearProgressIndicator(
                    value: (d['total_bytes'] ?? 0) == 0
                        ? 0
                        : ((d['total_bytes'] - d['free_bytes']) /
                            (d['total_bytes'] ?? 1)),
                  ),
                ],
              ),
            ),
        ]),
        const SizedBox(height: 20),
        CardSection(title: 'Logs', children: [
          const SizedBox(height: 4),
          _LogList(s: s),
        ]),
      ],
    );
  }

  String _uptime(dynamic secs) {
    final n = (secs ?? 0) as num;
    final d = Duration(seconds: n.toInt());
    if (d.inDays > 0) return '${d.inDays}d ${d.inHours % 24}h';
    if (d.inHours > 0) return '${d.inHours}h ${d.inMinutes % 60}m';
    return '${d.inMinutes}m ${d.inSeconds % 60}s';
  }
}

class _LogList extends StatefulWidget {
  final AppState s;
  const _LogList({required this.s});
  @override
  State<_LogList> createState() => _LogListState();
}

class _LogListState extends State<_LogList> {
  List<dynamic> _entries = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final e = await widget.s.fetchLogs(300);
    if (mounted) setState(() => _entries = e);
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Align(
          alignment: Alignment.centerRight,
          child: IconButton(
              onPressed: _load,
              icon: const Icon(Icons.refresh, size: 18),
              tooltip: 'Refresh'),
        ),
        Container(
          constraints: const BoxConstraints(maxHeight: 360),
          decoration: BoxDecoration(
            color: const Color(0xFF0A0A0C),
            borderRadius: BorderRadius.circular(8),
          ),
          child: ListView.builder(
            shrinkWrap: true,
            padding: const EdgeInsets.all(10),
            itemCount: _entries.length,
            itemBuilder: (context, i) {
              final e = _entries[i];
              final level = e['level'] ?? '';
              final color = switch (level) {
                'ERROR' => F.bad,
                'WARN' => F.warn,
                _ => const Color(0xFF8E8E93),
              };
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 1),
                child: Text.rich(
                  TextSpan(children: [
                    TextSpan(
                        text: '${e['ts']} ',
                        style: const TextStyle(color: Color(0xFF636366))),
                    TextSpan(
                        text: '${level.padRight(5)} ',
                        style: TextStyle(color: color)),
                    TextSpan(
                        text: e['message'] ?? '',
                        style: const TextStyle(color: Color(0xFFD1D1D6))),
                  ]),
                  style:
                      const TextStyle(fontFamily: 'monospace', fontSize: 11),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}
