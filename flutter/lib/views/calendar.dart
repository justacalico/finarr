import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../widgets.dart';

/// Upcoming episodes + releases grouped by day.
class CalendarView extends StatelessWidget {
  const CalendarView({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    final width = MediaQuery.sizeOf(context).width;
    // Group items by date.
    final byDay = <String, List<dynamic>>{};
    for (final it in s.calendarItems) {
      byDay.putIfAbsent(it['date'] ?? '', () => []).add(it);
    }
    final days = byDay.keys.toList()..sort();
    return Scaffold(
      body: RefreshIndicator(
        onRefresh: s.refreshCalendar,
        child: s.calendarItems.isEmpty
            ? const EmptyState(
                Icons.calendar_month_outlined, 'Nothing scheduled',
                message: 'Upcoming episodes and releases appear here.')
            : ListView(
                padding: EdgeInsets.all(width >= 900 ? 32 : 16),
                children: [
                  const SectionHeader('Calendar',
                      subtitle: 'Upcoming episodes and releases'),
                  for (final day in days) ...[
                    _DayHeader(date: day),
                    Card(
                      margin: const EdgeInsets.only(bottom: 16),
                      child: Column(
                        children: [
                          for (final it in byDay[day]!) _Row(item: it),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
      ),
    );
  }
}

class _DayHeader extends StatelessWidget {
  final String date;
  const _DayHeader({required this.date});

  @override
  Widget build(BuildContext context) {
    final d = DateTime.tryParse(date);
    final label = d == null
        ? date
        : '${DateFormat.EEEE().format(d)}, ${DateFormat.yMMMd().format(d)}';
    final today = DateTime.now();
    final isToday = d != null &&
        d.year == today.year &&
        d.month == today.month &&
        d.day == today.day;
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 8, top: 8),
      child: Text(
        isToday ? 'Today · $label' : label,
        style: TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w700,
          color: isToday ? Theme.of(context).colorScheme.primary : Theme.of(context).hintColor,
        ),
      ),
    );
  }
}

class _Row extends StatelessWidget {
  final Map<String, dynamic> item;
  const _Row({required this.item});

  @override
  Widget build(BuildContext context) {
    final kind = item['kind'];
    final title = kind == 'episode'
        ? '${item['series_title']} S${item['season'].toString().padLeft(2, '0')}E${item['episode'].toString().padLeft(2, '0')}'
        : item['title'] ?? '';
    final subtitle = kind == 'episode'
        ? (item['title'] ?? '')
        : kind == 'movie'
            ? (item['subtype'] == 'digital' ? 'Digital release' : 'In cinemas')
            : '';
    return ListTile(
      leading: Poster(url: item['poster_url'], title: title, width: 34),
      title: Text(title, maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: subtitle.isEmpty
          ? null
          : Text(subtitle,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 11)),
      trailing: StatusChip(item['status'] ?? ''),
      onTap: () {
        if (kind == 'episode' && item['series_id'] != null) {
          context.push('/series/${item['series_id']}');
        } else if (kind == 'movie') {
          context.push('/movie/${item['id']}');
        }
      },
    );
  }
}
