import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'api.dart';

/// The one application state object. Created once above the app widget;
/// every view reads through it and refreshes through it. Layout changes
/// must never recreate it.
class AppState extends ChangeNotifier {
  AppState() {
    _init();
  }

  final api = Api();

  // ---------------- boot / auth ----------------
  bool booted = false;
  bool setupRequired = false;
  bool get loggedIn => api.token != null;
  Map<String, dynamic>? user;
  String version = '';
  String instanceName = 'Finarr';
  String themeMode = 'system';
  String? serverUrl;

  Future<void> _init() async {
    final prefs = await SharedPreferences.getInstance();
    serverUrl = prefs.getString('server_url');
    if (serverUrl != null && serverUrl!.isNotEmpty) {
      api.baseUrl = serverUrl!;
    }
    api.token = prefs.getString('token');
    themeMode = prefs.getString('theme') ?? 'system';
    await checkStatus();
    if (loggedIn) {
      unawaited(refreshMe());
      unawaited(refreshAll());
      _startPolling();
    }
    booted = true;
    notifyListeners();
  }

  Future<void> checkStatus() async {
    try {
      final s = await api.get('/api/status');
      setupRequired = s['setup_required'] == true;
      version = s['version'] ?? '';
      instanceName = s['instance_name'] ?? 'Finarr';
    } catch (_) {}
    notifyListeners();
  }

  Future<void> refreshMe() async {
    try {
      user = await api.get('/api/auth/me');
    } on ApiException catch (e) {
      if (e.status == 401) {
        await logout();
        return;
      }
    } catch (_) {}
    notifyListeners();
  }

  Future<String?> login(String username, String password) async {
    try {
      final r = await api.post('/api/auth/login', {
        'username': username,
        'password': password,
      });
      api.token = r['token'];
      user = r['user'];
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('token', api.token!);
      unawaited(refreshAll());
      _startPolling();
      notifyListeners();
      return null;
    } on ApiException catch (e) {
      return e.message;
    } catch (e) {
      return e.toString();
    }
  }

  Future<String?> setup(String username, String password, String displayName,
      {Map<String, dynamic>? paths}) async {
    try {
      final r = await api.post('/api/setup', {
        'username': username,
        'password': password,
        'display_name': displayName,
        'paths': ?paths,
      });
      api.token = r['token'];
      user = r['user'];
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('token', api.token!);
      setupRequired = false;
      unawaited(refreshAll());
      _startPolling();
      notifyListeners();
      return null;
    } on ApiException catch (e) {
      return e.message;
    } catch (e) {
      return e.toString();
    }
  }

  Future<void> logout() async {
    try {
      await api.post('/api/auth/logout');
    } catch (_) {}
    api.token = null;
    user = null;
    _pollTimer?.cancel();
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove('token');
    notifyListeners();
  }

  Future<void> setServerUrl(String url) async {
    serverUrl = url;
    api.baseUrl = url;
    final prefs = await SharedPreferences.getInstance();
    if (url.isEmpty) {
      await prefs.remove('server_url');
    } else {
      await prefs.setString('server_url', url);
    }
    await checkStatus();
    notifyListeners();
  }

  Future<void> setThemeMode(String mode) async {
    themeMode = mode;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('theme', mode);
    notifyListeners();
  }

  // ---------------- data ----------------
  List<dynamic> torrents = [];
  Map<String, dynamic> speeds = {'download': 0, 'upload': 0};
  List<dynamic> movies = [];
  List<dynamic> seriesList = [];
  List<dynamic> artists = [];
  List<dynamic> requests = [];
  List<dynamic> queue = [];
  List<dynamic> history = [];
  List<dynamic> indexers = [];
  List<dynamic> clients = [];
  List<dynamic> users = [];
  List<dynamic> calendarItems = [];
  Map<String, dynamic> wanted = {};
  Map<String, dynamic> settings = {};
  Map<String, dynamic> systemInfo = {};
  List<dynamic> disk = [];

  Future<void> refreshAll() async {
    if (!loggedIn) return;
    await Future.wait([
      refreshTorrents(),
      refreshLibraries(),
      refreshQueue(),
      refreshRequests(),
      refreshWanted(),
      refreshIndexers(),
      refreshClients(),
      refreshHistory(),
      refreshCalendar(),
      refreshSettings(),
      refreshSystem(),
    ]);
  }

  bool get isAdmin => user?['role'] == 'admin';

  Future<void> refreshTorrents() async {
    try {
      final r = await api.get('/api/torrents');
      torrents = r['torrents'] ?? [];
      speeds = r['speeds'] ?? speeds;
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshLibraries() async {
    try {
      final r = await Future.wait([
        api.get('/api/movies'),
        api.get('/api/series'),
        api.get('/api/artists'),
      ]);
      movies = r[0]['movies'] ?? [];
      seriesList = r[1]['series'] ?? [];
      artists = r[2]['artists'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshQueue() async {
    try {
      final r = await api.get('/api/activity/queue');
      queue = r['items'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshRequests() async {
    try {
      final r = await api.get('/api/requests');
      requests = r['requests'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshWanted() async {
    try {
      wanted = await api.get('/api/wanted');
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshIndexers() async {
    try {
      final r = await api.get('/api/indexers');
      indexers = r['indexers'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshClients() async {
    try {
      final r = await api.get('/api/clients');
      clients = r['clients'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshUsers() async {
    try {
      final r = await api.get('/api/users');
      users = r['users'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshHistory() async {
    try {
      final r = await api.get('/api/activity/history', {'limit': '150'});
      history = r['history'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshCalendar() async {
    try {
      final r = await api.get('/api/calendar');
      calendarItems = r['items'] ?? [];
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshSettings() async {
    try {
      settings = await api.get('/api/settings');
      notifyListeners();
    } catch (_) {}
  }

  Future<void> refreshSystem() async {
    try {
      systemInfo = await api.get('/api/system/info');
      disk = await api.get('/api/system/disk');
      notifyListeners();
    } catch (_) {}
  }

  Future<List<dynamic>> fetchLogs(int limit) async {
    try {
      final r = await api.get('/api/system/logs', {'limit': '$limit'});
      return r['entries'] ?? [];
    } catch (_) {
      return [];
    }
  }

  // ---------------- polling ----------------
  Timer? _pollTimer;

  void _startPolling() {
    _pollTimer?.cancel();
    // Fast lane for the queue view; the rest refreshes lazily per-view.
    _pollTimer = Timer.periodic(const Duration(seconds: 3), (_) {
      if (!loggedIn) return;
      unawaited(refreshTorrents());
      unawaited(refreshQueue());
    });
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }
}
