import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';

/// First-run wizard: admin account + (optional) media paths.
class SetupView extends StatefulWidget {
  const SetupView({super.key});

  @override
  State<SetupView> createState() => _SetupViewState();
}

class _SetupViewState extends State<SetupView> {
  final _user = TextEditingController();
  final _pass = TextEditingController();
  final _pass2 = TextEditingController();
  final _name = TextEditingController();
  final _movies = TextEditingController(text: 'data/library/movies');
  final _series = TextEditingController(text: 'data/library/series');
  final _music = TextEditingController(text: 'data/library/music');
  final _downloads = TextEditingController(text: 'data/downloads');
  bool _busy = false;
  String? _error;
  int _step = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 460),
          child: Card(
            margin: const EdgeInsets.all(24),
            child: Padding(
              padding: const EdgeInsets.all(28),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          color: F.accent,
                          borderRadius: BorderRadius.circular(10),
                        ),
                        alignment: Alignment.center,
                        child: const Text('F',
                            style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.w800,
                                fontSize: 18)),
                      ),
                      const SizedBox(width: 12),
                      const Expanded(
                        child: Text('Welcome to Finarr',
                            style: TextStyle(
                                fontSize: 20, fontWeight: FontWeight.w700)),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    _step == 0
                        ? 'Create the administrator account.'
                        : 'Where should media live? You can change this later in Settings.',
                    style: TextStyle(color: Theme.of(context).hintColor),
                  ),
                  const SizedBox(height: 20),
                  if (_step == 0) ..._accountStep() else ..._pathsStep(),
                  if (_error != null) ...[
                    const SizedBox(height: 12),
                    Text(_error!, style: const TextStyle(color: F.bad, fontSize: 13)),
                  ],
                  const SizedBox(height: 20),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      if (_step == 1)
                        TextButton(
                          onPressed: () => setState(() => _step = 0),
                          child: const Text('Back'),
                        ),
                      const SizedBox(width: 8),
                      FilledButton(
                        onPressed: _busy
                            ? null
                            : (_step == 0 ? _next : _finish),
                        child: _busy
                            ? const SizedBox(
                                width: 18,
                                height: 18,
                                child: CircularProgressIndicator(
                                    strokeWidth: 2, color: Colors.white))
                            : Text(_step == 0 ? 'Continue' : 'Finish setup'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  List<Widget> _accountStep() => [
        TextField(
          controller: _user,
          decoration: const InputDecoration(labelText: 'Username'),
          autofocus: true,
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _name,
          decoration: const InputDecoration(labelText: 'Display name (optional)'),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _pass,
          decoration: const InputDecoration(labelText: 'Password'),
          obscureText: true,
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _pass2,
          decoration: const InputDecoration(labelText: 'Confirm password'),
          obscureText: true,
          onSubmitted: (_) => _next(),
        ),
      ];

  List<Widget> _pathsStep() => [
        TextField(
          controller: _downloads,
          decoration: const InputDecoration(
              labelText: 'Downloads folder',
              helperText: 'Where the engine stores torrent data'),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _movies,
          decoration: const InputDecoration(labelText: 'Movies library'),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _series,
          decoration: const InputDecoration(labelText: 'Series library'),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _music,
          decoration: const InputDecoration(labelText: 'Music library'),
        ),
      ];

  void _next() {
    if (_user.text.trim().isEmpty) {
      setState(() => _error = 'Username is required');
      return;
    }
    if (_pass.text.length < 8) {
      setState(() => _error = 'Password needs at least 8 characters');
      return;
    }
    if (_pass.text != _pass2.text) {
      setState(() => _error = 'Passwords do not match');
      return;
    }
    setState(() {
      _error = null;
      _step = 1;
    });
  }

  Future<void> _finish() async {
    final s = context.read<AppState>();
    setState(() => _busy = true);
    final err = await s.setup(
      _user.text.trim(),
      _pass.text,
      _name.text.trim().isEmpty ? _user.text.trim() : _name.text.trim(),
      paths: {
        'downloads_dir': _downloads.text.trim(),
        'movies_root': _movies.text.trim(),
        'series_root': _series.text.trim(),
        'music_root': _music.text.trim(),
        'import_mode': 'hardlink',
      },
    );
    if (!mounted) return;
    setState(() {
      _busy = false;
      _error = err;
    });
  }
}
