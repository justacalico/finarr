import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';

import 'app_state.dart';
import 'theme.dart';

// ------------------------------ formatting ------------------------------

String fmtBytes(num bytes, {int decimals = 1}) {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  var i = 0;
  var v = bytes.toDouble();
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return '${v.toStringAsFixed(i == 0 ? 0 : decimals)} ${units[i]}';
}

String fmtSpeed(num bytesPerSec) => '${fmtBytes(bytesPerSec)}/s';

String fmtEta(num? seconds) {
  if (seconds == null || seconds <= 0 || seconds >= 864000) return '-';
  final d = Duration(seconds: seconds.toInt());
  if (d.inDays > 0) return '${d.inDays}d ${d.inHours % 24}h';
  if (d.inHours > 0) return '${d.inHours}h ${d.inMinutes % 60}m';
  if (d.inMinutes > 0) return '${d.inMinutes}m ${d.inSeconds % 60}s';
  return '${d.inSeconds}s';
}

String fmtDate(String? iso) {
  if (iso == null || iso.isEmpty) return '-';
  final d = DateTime.tryParse(iso);
  if (d == null) return iso;
  return DateFormat.yMMMd().format(d.toLocal());
}

String fmtDateTime(String? iso) {
  if (iso == null || iso.isEmpty) return '-';
  final d = DateTime.tryParse(iso);
  if (d == null) return iso;
  return DateFormat.yMMMd().add_Hm().format(d.toLocal());
}

String fmtPct(double p) => '${(p * 100).clamp(0, 100).toStringAsFixed(1)}%';

// ------------------------------ atoms ------------------------------

/// A poster image with a graceful monogram fallback.
class Poster extends StatelessWidget {
  final String? url;
  final String title;
  final double aspect; // width / height; 2/3 for posters, square for music
  final double? width;
  final double radius;

  const Poster({
    super.key,
    this.url,
    required this.title,
    this.aspect = 2 / 3,
    this.width,
    this.radius = 8,
  });

  @override
  Widget build(BuildContext context) {
    final api = context.read<AppState>().api;
    final proxied = api.imageUrl(url);
    final fallback = Container(
      color: F.accentSoft,
      alignment: Alignment.center,
      child: Text(
        title.trim().isEmpty ? '?' : title.trim()[0].toUpperCase(),
        style: TextStyle(
          color: F.accent,
          fontSize: (width ?? 120) / 4,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
    return ClipRRect(
      borderRadius: BorderRadius.circular(radius),
      child: AspectRatio(
        aspectRatio: aspect,
        child: proxied.isEmpty
            ? fallback
            : Image.network(
                proxied,
                fit: BoxFit.cover,
                width: width,
                headers: {
                  // The image proxy requires auth; Image.network doesn't
                  // send our Authorization header on its own.
                  if (api.token != null)
                    'authorization': 'Bearer ${api.token}',
                },
                loadingBuilder: (c, w, progress) =>
                    progress == null ? w : fallback,
                errorBuilder: (_, _, _) => fallback,
              ),
      ),
    );
  }
}

class StatusChip extends StatelessWidget {
  final String status;
  const StatusChip(this.status, {super.key});

  @override
  Widget build(BuildContext context) {
    final (label, color) = switch (status) {
      'imported' => ('In Library', F.ok),
      'downloading' => ('Downloading', F.info),
      'missing' => ('Missing', F.warn),
      'unaired' => ('Unaired', Colors.grey),
      'pending' => ('Pending', F.warn),
      'approved' => ('Approved', F.info),
      'declined' => ('Declined', F.bad),
      'fulfilled' => ('Fulfilled', F.ok),
      'live' => ('Downloading', F.info),
      'paused' => ('Paused', Colors.grey),
      'completed' => ('Completed', F.ok),
      'import_failed' => ('Import Failed', F.bad),
      'removed' => ('Removed', Colors.grey),
      'queued' => ('Queued', Colors.grey),
      'initializing' => ('Fetching info', F.info),
      'error' => ('Error', F.bad),
      _ => (status.isEmpty ? 'Unknown' : status, Colors.grey),
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Text(
        label,
        style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.w600),
      ),
    );
  }
}

class SectionHeader extends StatelessWidget {
  final String title;
  final String? subtitle;
  final Widget? trailing;
  const SectionHeader(this.title, {super.key, this.subtitle, this.trailing});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title,
                    style: const TextStyle(
                        fontSize: 20, fontWeight: FontWeight.w600)),
                if (subtitle != null) ...[
                  const SizedBox(height: 2),
                  Text(subtitle!,
                      style: TextStyle(
                          fontSize: 13, color: Theme.of(context).hintColor)),
                ],
              ],
            ),
          ),
          ?trailing,
        ],
      ),
    );
  }
}

class EmptyState extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? message;
  final Widget? action;
  const EmptyState(this.icon, this.title,
      {super.key, this.message, this.action});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(48),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 44, color: Theme.of(context).hintColor),
            const SizedBox(height: 16),
            Text(title,
                style:
                    const TextStyle(fontSize: 17, fontWeight: FontWeight.w600)),
            if (message != null) ...[
              const SizedBox(height: 6),
              Text(message!,
                  textAlign: TextAlign.center,
                  style:
                      TextStyle(fontSize: 13, color: Theme.of(context).hintColor)),
            ],
            if (action != null) ...[
              const SizedBox(height: 20),
              action!,
            ],
          ],
        ),
      ),
    );
  }
}

Future<bool> confirm(BuildContext context,
    {required String title,
    required String message,
    String confirmLabel = 'Confirm',
    bool destructive = false}) async {
  final res = await showDialog<bool>(
    context: context,
    builder: (c) => AlertDialog(
      title: Text(title),
      content: Text(message),
      actions: [
        TextButton(
            onPressed: () => Navigator.pop(c, false),
            child: const Text('Cancel')),
        FilledButton(
          style: destructive
              ? FilledButton.styleFrom(backgroundColor: F.bad)
              : null,
          onPressed: () => Navigator.pop(c, true),
          child: Text(confirmLabel),
        ),
      ],
    ),
  );
  return res == true;
}

void snack(BuildContext context, String message, {bool error = false}) {
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
    content: Text(message),
    backgroundColor: error ? F.bad : null,
  ));
}

/// A labelled card block used across settings and detail pages.
class CardSection extends StatelessWidget {
  final String? title;
  final List<Widget> children;
  const CardSection({super.key, this.title, required this.children});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (title != null)
          Padding(
            padding: const EdgeInsets.only(left: 4, bottom: 8),
            child: Text(title!,
                style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    color: Theme.of(context).hintColor)),
          ),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: children,
            ),
          ),
        ),
      ],
    );
  }
}

/// Small inline stat for headers/detail rows.
class Stat extends StatelessWidget {
  final String label;
  final String value;
  const Stat(this.label, this.value, {super.key});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label,
            style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor)),
        const SizedBox(height: 2),
        Text(value,
            style:
                const TextStyle(fontSize: 15, fontWeight: FontWeight.w600)),
      ],
    );
  }
}

/// Three-way delete choice: returns 'keep', 'files' or null (dismissed).
Future<String?> confirmDelete(BuildContext context,
    {required String title, String? message}) {
  return showDialog<String>(
    context: context,
    builder: (c) => AlertDialog(
      title: Text(title),
      content: Text(message ?? 'Keep the files on disk or delete them too?'),
      actions: [
        TextButton(
            onPressed: () => Navigator.pop(c), child: const Text('Cancel')),
        TextButton(
            onPressed: () => Navigator.pop(c, 'keep'),
            child: const Text('Keep files')),
        FilledButton(
          style: FilledButton.styleFrom(backgroundColor: F.bad),
          onPressed: () => Navigator.pop(c, 'files'),
          child: const Text('Delete files'),
        ),
      ],
    ),
  );
}
