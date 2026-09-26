import 'package:finarr/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('fmtBytes', () {
    test('formats units', () {
      expect(fmtBytes(0), '0 B');
      expect(fmtBytes(-5), '0 B');
      expect(fmtBytes(512), '512 B');
      expect(fmtBytes(1024), '1.0 KiB');
      expect(fmtBytes(5 * 1024 * 1024), '5.0 MiB');
      expect(fmtBytes(2 * 1024 * 1024 * 1024), '2.0 GiB');
      expect(fmtBytes(3 * 1024 * 1024 * 1024 * 1024), '3.0 TiB');
    });
  });

  group('fmtSpeed', () {
    test('appends per-second', () {
      expect(fmtSpeed(0), '0 B/s');
      expect(fmtSpeed(1024 * 100), '100.0 KiB/s');
    });
  });

  group('fmtEta', () {
    test('formats durations', () {
      expect(fmtEta(null), '-');
      expect(fmtEta(0), '-');
      expect(fmtEta(-3), '-');
      expect(fmtEta(45), '45s');
      expect(fmtEta(95), '1m 35s');
      expect(fmtEta(3661), '1h 1m');
      expect(fmtEta(90061), '1d 1h');
      expect(fmtEta(864000), '-');
    });
  });

  group('fmtDate / fmtDateTime', () {
    test('handles empty and invalid', () {
      expect(fmtDate(null), '-');
      expect(fmtDate(''), '-');
      expect(fmtDate('not a date'), 'not a date');
      expect(fmtDateTime(null), '-');
    });

    test('formats ISO dates', () {
      expect(fmtDate('2025-03-15'), contains('2025'));
      expect(fmtDateTime('2025-03-15T14:30:00'), contains('2025'));
    });
  });

  group('fmtPct', () {
    test('clamps and formats', () {
      expect(fmtPct(0), '0.0%');
      expect(fmtPct(0.5), '50.0%');
      expect(fmtPct(1.0), '100.0%');
      expect(fmtPct(1.5), '100.0%');
      expect(fmtPct(-0.2), '0.0%');
    });
  });
}
