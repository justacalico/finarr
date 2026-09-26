import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';
import '../widgets.dart';
import 'activity.dart';
import 'calendar.dart';
import 'home.dart';
import 'movies.dart';
import 'music.dart';
import 'requests.dart';
import 'series.dart';
import 'settings.dart';
import 'torrents.dart';
import 'wanted.dart';

/// Adaptive root: NavigationRail on wide layouts, NavigationBar on
/// compact ones. Section bodies are built lazily and kept alive.
class ShellView extends StatefulWidget {
  final int initialIndex;
  const ShellView({super.key, this.initialIndex = 0});

  @override
  State<ShellView> createState() => _ShellViewState();
}

class _ShellViewState extends State<ShellView> {
  late int _index = widget.initialIndex;
  final Map<int, Widget> _built = {};

  static const _sections = [
    ('Home', Icons.home_outlined, Icons.home),
    ('Movies', Icons.movie_outlined, Icons.movie),
    ('Series', Icons.tv_outlined, Icons.tv),
    ('Music', Icons.album_outlined, Icons.album),
    ('Torrents', Icons.download_outlined, Icons.download),
    ('Requests', Icons.bookmark_add_outlined, Icons.bookmark_add),
    ('Activity', Icons.swap_vert_outlined, Icons.swap_vert),
    ('Calendar', Icons.calendar_month_outlined, Icons.calendar_month),
    ('Wanted', Icons.search_outlined, Icons.search),
    ('Settings', Icons.settings_outlined, Icons.settings),
  ];

  Widget _view(int i) => _built.putIfAbsent(i, () {
        return switch (i) {
          0 => const HomeView(),
          1 => const MoviesView(),
          2 => const SeriesView(),
          3 => const MusicView(),
          4 => const TorrentsView(),
          5 => const RequestsView(),
          6 => const ActivityView(),
          7 => const CalendarView(),
          8 => const WantedView(),
          _ => const SettingsView(),
        };
      });

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= 900;
    final s = context.watch<AppState>();
    final pending = s.requests.where((r) => r['status'] == 'pending').length;

    return Scaffold(
      appBar: wide
          ? null
          : AppBar(
              title: Text(_sections[_index].$1),
              centerTitle: false,
              actions: [
                _SpeedBadge(speeds: s.speeds),
                const SizedBox(width: 8),
              ],
            ),
      body: Row(
        children: [
          if (wide)
            NavigationRail(
              selectedIndex: _index,
              onDestinationSelected: (i) => setState(() => _index = i),
              labelType: NavigationRailLabelType.all,
              leading: Padding(
                padding: const EdgeInsets.only(top: 8, bottom: 24),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 28,
                      height: 28,
                      decoration: BoxDecoration(
                        color: F.accent,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      alignment: Alignment.center,
                      child: const Text('F',
                          style: TextStyle(
                              color: Colors.white,
                              fontWeight: FontWeight.w800,
                              fontSize: 15)),
                    ),
                    const SizedBox(width: 10),
                    Text(s.instanceName,
                        style: const TextStyle(
                            fontWeight: FontWeight.w700, fontSize: 15)),
                  ],
                ),
              ),
              trailing: Expanded(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    _SpeedBadge(speeds: s.speeds),
                    const SizedBox(height: 8),
                    _UserMenu(user: s.user),
                    const SizedBox(height: 12),
                  ],
                ),
              ),
              destinations: [
                for (var i = 0; i < _sections.length; i++)
                  NavigationRailDestination(
                    icon: Badge.count(
                      count: i == 5 && s.isAdmin ? pending : 0,
                      isLabelVisible: i == 5 && s.isAdmin && pending > 0,
                      child: Icon(_sections[i].$2),
                    ),
                    selectedIcon: Icon(_sections[i].$3),
                    label: Text(_sections[i].$1),
                  ),
              ],
            ),
          if (wide) const VerticalDivider(width: 1),
          Expanded(child: _view(_index)),
        ],
      ),
      bottomNavigationBar: wide
          ? null
          : NavigationBar(
              selectedIndex: _index,
              onDestinationSelected: (i) => setState(() => _index = i),
              destinations: [
                for (var i = 0; i < _sections.length; i++)
                  NavigationDestination(
                    icon: Badge.count(
                      count: i == 5 && s.isAdmin ? pending : 0,
                      isLabelVisible: i == 5 && s.isAdmin && pending > 0,
                      child: Icon(_sections[i].$2),
                    ),
                    selectedIcon: Icon(_sections[i].$3),
                    label: _sections[i].$1,
                  ),
              ],
            ),
    );
  }
}

class _SpeedBadge extends StatelessWidget {
  final Map<String, dynamic> speeds;
  const _SpeedBadge({required this.speeds});

  @override
  Widget build(BuildContext context) {
    final dl = speeds['download'] ?? 0;
    final ul = speeds['upload'] ?? 0;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.arrow_downward, size: 13, color: F.info),
          const SizedBox(width: 3),
          Text(fmtSpeed(dl), style: const TextStyle(fontSize: 11)),
          const SizedBox(width: 8),
          const Icon(Icons.arrow_upward, size: 13, color: F.ok),
          const SizedBox(width: 3),
          Text(fmtSpeed(ul), style: const TextStyle(fontSize: 11)),
        ],
      ),
    );
  }
}

class _UserMenu extends StatelessWidget {
  final Map<String, dynamic>? user;
  const _UserMenu({required this.user});

  @override
  Widget build(BuildContext context) {
    final name = user?['display_name'] ?? user?['username'] ?? '?';
    return PopupMenuButton<String>(
      tooltip: 'Account',
      onSelected: (v) async {
        if (v == 'logout') {
          await context.read<AppState>().logout();
        }
      },
      itemBuilder: (_) => [
        PopupMenuItem(
          enabled: false,
          child: Text('$name · ${user?['role'] ?? ''}',
              style: const TextStyle(fontSize: 12)),
        ),
        const PopupMenuItem(value: 'logout', child: Text('Sign out')),
      ],
      child: CircleAvatar(
        radius: 15,
        backgroundColor: F.accentSoft,
        child: Text(
          name.isEmpty ? '?' : name[0].toUpperCase(),
          style: const TextStyle(color: F.accent, fontSize: 13),
        ),
      ),
    );
  }
}
