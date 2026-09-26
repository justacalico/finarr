import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;

/// Thin API client. On web the base URL is empty (same origin); on
/// mobile/desktop the user can point the app at any Finarr server.
class Api {
  String baseUrl;
  String? token;
  final http.Client client;

  Api({this.baseUrl = '', this.token, http.Client? client})
      : client = client ?? http.Client();

  Uri _u(String path, [Map<String, String>? q]) {
    final qp = (q == null || q.isEmpty) ? null : q;
    if (baseUrl.isEmpty) {
      return Uri(path: path, queryParameters: qp);
    }
    final base = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    return Uri.parse('$base$path').replace(queryParameters: qp);
  }

  Map<String, String> get _headers => {
        'content-type': 'application/json',
        if (token != null) 'authorization': 'Bearer $token',
      };

  void _check(http.Response r) {
    if (r.statusCode >= 400) {
      String msg = 'HTTP ${r.statusCode}';
      try {
        final j = jsonDecode(r.body);
        if (j is Map && j['error'] != null) msg = j['error'];
      } catch (_) {}
      throw ApiException(r.statusCode, msg);
    }
  }

  Future<dynamic> get(String path, [Map<String, String>? q]) async {
    final r = await client.get(_u(path, q), headers: _headers);
    _check(r);
    return r.body.isEmpty ? null : jsonDecode(r.body);
  }

  Future<dynamic> post(String path, [Object? body]) async {
    final r = await client.post(_u(path),
        headers: _headers, body: body == null ? null : jsonEncode(body));
    _check(r);
    return r.body.isEmpty ? null : jsonDecode(r.body);
  }

  Future<dynamic> put(String path, [Object? body]) async {
    final r = await client.put(_u(path),
        headers: _headers, body: body == null ? null : jsonEncode(body));
    _check(r);
    return r.body.isEmpty ? null : jsonDecode(r.body);
  }

  Future<dynamic> patch(String path, [Object? body]) async {
    final r = await client.patch(_u(path),
        headers: _headers, body: body == null ? null : jsonEncode(body));
    _check(r);
    return r.body.isEmpty ? null : jsonDecode(r.body);
  }

  Future<dynamic> delete(String path, [Map<String, String>? q]) async {
    final r = await client.delete(_u(path, q), headers: _headers);
    _check(r);
    return r.body.isEmpty ? null : jsonDecode(r.body);
  }

  /// Multipart .torrent upload.
  Future<dynamic> addTorrentFile(Uint8List bytes, String filename,
      {String category = ''}) async {
    final req = http.MultipartRequest('POST', _u('/api/torrents/file'));
    req.headers.addAll({
      if (token != null) 'authorization': 'Bearer $token',
    });
    req.files.add(http.MultipartFile.fromBytes('torrent', bytes, filename: filename));
    if (category.isNotEmpty) req.fields['category'] = category;
    final streamed = await req.send();
    final r = await http.Response.fromStream(streamed);
    _check(r);
    return r.body.isEmpty ? null : jsonDecode(r.body);
  }

  /// Route remote images through the backend proxy (CORS-safe on web).
  String imageUrl(String? remote) {
    if (remote == null || remote.isEmpty) return '';
    if (remote.startsWith('http')) {
      return _u('/api/proxy/image', {'url': remote}).toString();
    }
    return remote;
  }
}

class ApiException implements Exception {
  final int status;
  final String message;
  ApiException(this.status, this.message);
  @override
  String toString() => message;
}
