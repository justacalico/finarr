import 'dart:convert';

import 'package:finarr/api.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  group('Api', () {
    test('get decodes JSON and sends auth header', () async {
      Map<String, String>? seenAuth;
      final client = MockClient((req) async {
        seenAuth = req.headers['authorization'] == null
            ? null
            : {'auth': req.headers['authorization']!};
        expect(req.url.path, '/api/status');
        return http.Response(
            jsonEncode({'version': '1.0.0', 'setup_required': false}), 200);
      });
      final api = Api(token: 'tok123', client: client);
      final r = await api.get('/api/status');
      expect(r['version'], '1.0.0');
      expect(seenAuth?['auth'], 'Bearer tok123');
    });

    test('error surfaces server message', () async {
      final client = MockClient(
          (_) async => http.Response(jsonEncode({'error': 'not allowed'}), 403));
      final api = Api(client: client);
      try {
        await api.get('/api/admin');
        fail('should throw');
      } on ApiException catch (e) {
        expect(e.status, 403);
        expect(e.message, 'not allowed');
      }
    });

    test('error falls back to status text', () async {
      final client = MockClient((_) async => http.Response('bad', 500));
      final api = Api(client: client);
      try {
        await api.post('/api/x', {'a': 1});
        fail('should throw');
      } on ApiException catch (e) {
        expect(e.status, 500);
        expect(e.message, contains('500'));
      }
    });

    test('query params are encoded', () async {
      String? rawQuery;
      final client = MockClient((req) async {
        rawQuery = req.url.query;
        return http.Response('{}', 200);
      });
      final api = Api(client: client);
      await api.get('/api/lookup/movies', {'q': 'hello world'});
      expect(rawQuery, contains('q=hello'));
      expect(rawQuery, contains('world'));
    });

    test('base url is prefixed for remote servers', () async {
      String? host;
      String? path;
      final client = MockClient((req) async {
        host = req.url.host;
        path = req.url.path;
        return http.Response('{}', 200);
      });
      final api = Api(baseUrl: 'http://nas.local:8787/', client: client);
      await api.get('/api/status');
      expect(host, 'nas.local');
      expect(path, '/api/status');
    });

    test('imageUrl proxies remote urls', () {
      final api = Api();
      final u = api.imageUrl('https://img.example.com/p.jpg');
      expect(u, contains('/api/proxy/image'));
      expect(u, contains('img.example.com'));
      expect(api.imageUrl(null), '');
      expect(api.imageUrl('/local.png'), '/local.png');
    });
  });
}
