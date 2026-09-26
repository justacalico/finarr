import 'package:finarr/views/home.dart';
import 'package:finarr/widgets.dart';
import 'package:finarr/views/login.dart';
import 'package:finarr/views/setup.dart';
import 'package:finarr/views/shell.dart';
import 'package:finarr/views/activity.dart';
import 'package:finarr/views/calendar.dart';
import 'package:finarr/views/movies.dart';
import 'package:finarr/views/music.dart';
import 'package:finarr/views/requests.dart';
import 'package:finarr/views/series.dart';
import 'package:finarr/views/settings.dart';
import 'package:finarr/views/torrents.dart';
import 'package:finarr/views/wanted.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'helpers.dart';

void main() {
  const compact = Size(390, 844);
  const wide = Size(1280, 800);

  group('screens render', () {
    final cases = <String, Widget Function()>{
      'home': () => const HomeView(),
      'movies': () => const MoviesView(),
      'series': () => const SeriesView(),
      'music': () => const MusicView(),
      'torrents': () => const TorrentsView(),
      'requests': () => const RequestsView(),
      'activity': () => const ActivityView(),
      'calendar': () => const CalendarView(),
      'wanted': () => const WantedView(),
      'settings': () => const SettingsView(),
    };

    for (final entry in cases.entries) {
      for (final (name, size) in [('compact', compact), ('wide', wide)]) {
        testWidgets('${entry.key} $name', (tester) async {
          await pumpScreen(tester, entry.value(), size: size);
          await expectLater(find.byType(entry.value().runtimeType),
              matchesGoldenFile('goldens/${entry.key}_$name.png'));
        });
      }
    }

    testWidgets('login compact', (tester) async {
      await pumpScreen(tester, const LoginView(), size: compact);
      await expectLater(
          find.byType(LoginView),
          matchesGoldenFile('goldens/login_compact.png'));
    });

    testWidgets('setup compact', (tester) async {
      await pumpScreen(tester, const SetupView(), size: compact);
      await expectLater(
          find.byType(SetupView),
          matchesGoldenFile('goldens/setup_compact.png'));
    });

    testWidgets('shell wide', (tester) async {
      await pumpScreen(tester, const ShellView(), size: wide);
      await expectLater(
          find.byType(ShellView),
          matchesGoldenFile('goldens/shell_wide.png'));
    });

    testWidgets('shell compact', (tester) async {
      await pumpScreen(tester, const ShellView(), size: compact);
      await expectLater(
          find.byType(ShellView),
          matchesGoldenFile('goldens/shell_compact.png'));
    });
  });

  group('widgets', () {
    testWidgets('poster shows monogram without url', (tester) async {
      await pumpScreen(
          tester,
          const Poster(url: null, title: 'Test Title', width: 80));
      expect(find.text('T'), findsOneWidget);
    });
  });
}
