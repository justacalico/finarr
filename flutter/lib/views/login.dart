import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../app_state.dart';
import '../theme.dart';

class LoginView extends StatefulWidget {
  const LoginView({super.key});

  @override
  State<LoginView> createState() => _LoginViewState();
}

class _LoginViewState extends State<LoginView> {
  final _user = TextEditingController();
  final _pass = TextEditingController();
  final _server = TextEditingController();
  bool _busy = false;
  String? _error;
  bool _showServer = false;

  @override
  void initState() {
    super.initState();
    final url = context.read<AppState>().serverUrl;
    if (url != null) _server.text = url;
  }

  @override
  void dispose() {
    _user.dispose();
    _pass.dispose();
    _server.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final s = context.watch<AppState>();
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 380),
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
                      Text(s.instanceName,
                          style: const TextStyle(
                              fontSize: 22, fontWeight: FontWeight.w700)),
                    ],
                  ),
                  const SizedBox(height: 24),
                  TextField(
                    controller: _user,
                    decoration: const InputDecoration(
                        labelText: 'Username', prefixIcon: Icon(Icons.person_outline)),
                    textInputAction: TextInputAction.next,
                    autofocus: true,
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _pass,
                    decoration: const InputDecoration(
                        labelText: 'Password', prefixIcon: Icon(Icons.lock_outline)),
                    obscureText: true,
                    onSubmitted: (_) => _submit(),
                  ),
                  if (_showServer) ...[
                    const SizedBox(height: 12),
                    TextField(
                      controller: _server,
                      decoration: const InputDecoration(
                          labelText: 'Server URL',
                          hintText: 'http://192.168.1.10:8787',
                          prefixIcon: Icon(Icons.dns_outlined)),
                      keyboardType: TextInputType.url,
                    ),
                  ],
                  if (_error != null) ...[
                    const SizedBox(height: 12),
                    Text(_error!, style: const TextStyle(color: F.bad, fontSize: 13)),
                  ],
                  const SizedBox(height: 20),
                  FilledButton(
                    onPressed: _busy ? null : _submit,
                    child: _busy
                        ? const SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator(
                                strokeWidth: 2, color: Colors.white))
                        : const Text('Sign in'),
                  ),
                  TextButton(
                    onPressed: () =>
                        setState(() => _showServer = !_showServer),
                    child: Text(
                        _showServer ? 'Same server as page' : 'Different server?'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Future<void> _submit() async {
    final s = context.read<AppState>();
    if (_showServer && _server.text.trim() != (s.serverUrl ?? '')) {
      await s.setServerUrl(_server.text.trim());
    }
    if (!mounted) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    final err = await s.login(_user.text.trim(), _pass.text);
    if (!mounted) return;
    setState(() {
      _busy = false;
      _error = err;
    });
  }
}
