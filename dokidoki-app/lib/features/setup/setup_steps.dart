import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_timezone/flutter_timezone.dart';
import 'package:go_router/go_router.dart';

import '../../core/api/auth_api.dart';
import '../../core/api/providers.dart';
import '../../core/auth/providers.dart';
import '../../core/models/api_error.dart';

class SetupServerStep extends ConsumerStatefulWidget {
  const SetupServerStep({
    super.key,
    required this.onNext,
  });

  final VoidCallback onNext;

  @override
  ConsumerState<SetupServerStep> createState() => _SetupServerStepState();
}

class _SetupServerStepState extends ConsumerState<SetupServerStep> {
  final _controller = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _testing = false;
  bool _connectionOk = false;
  String? _statusMessage;

  @override
  void initState() {
    super.initState();
    final serverUrl = ref.read(authConfigProvider).value?.serverUrl;
    if (serverUrl != null) {
      _controller.text = serverUrl;
      _connectionOk = true;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _testConnection() async {
    if (!_formKey.currentState!.validate()) {
      return;
    }

    setState(() {
      _testing = true;
      _connectionOk = false;
      _statusMessage = null;
    });

    final url = normalizeServerUrl(_controller.text);
    try {
      await testServerConnection(url);
      if (mounted) {
        setState(() {
          _connectionOk = true;
          _statusMessage = '连接成功';
        });
      }
    } on ApiException catch (error) {
      if (mounted) {
        setState(() {
          _statusMessage = error.error.message;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _testing = false);
      }
    }
  }

  Future<void> _saveAndNext() async {
    if (!_connectionOk) {
      return;
    }

    final url = normalizeServerUrl(_controller.text);
    await ref.read(authConfigProvider.notifier).setServerUrl(url);
    widget.onNext();
  }

  @override
  Widget build(BuildContext context) {
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '连接服务器',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          Text(
            '输入自部署 Dokidoki 后端地址',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
          const SizedBox(height: 24),
          TextFormField(
            controller: _controller,
            decoration: const InputDecoration(
              labelText: 'Server URL',
              hintText: 'http://192.168.1.10:8080',
              border: OutlineInputBorder(),
            ),
            keyboardType: TextInputType.url,
            autovalidateMode: AutovalidateMode.onUserInteraction,
            validator: (value) {
              if (value == null || value.trim().isEmpty) {
                return '请输入服务器地址';
              }
              if (!isValidServerUrl(value)) {
                return '请输入有效的 http/https 地址';
              }
              return null;
            },
            onChanged: (_) {
              if (_connectionOk || _statusMessage != null) {
                setState(() {
                  _connectionOk = false;
                  _statusMessage = null;
                });
              }
            },
          ),
          if (_statusMessage != null) ...[
            const SizedBox(height: 12),
            Text(
              _statusMessage!,
              style: TextStyle(
                color: _connectionOk
                    ? Theme.of(context).colorScheme.primary
                    : Theme.of(context).colorScheme.error,
              ),
            ),
          ],
          const SizedBox(height: 24),
          Row(
            children: [
              Expanded(
                child: OutlinedButton(
                  onPressed: _testing ? null : _testConnection,
                  child: _testing
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('测试连接'),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: FilledButton(
                  onPressed: _connectionOk ? _saveAndNext : null,
                  child: const Text('下一步'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

enum AuthTab { register, login }

class SetupAuthStep extends ConsumerStatefulWidget {
  const SetupAuthStep({super.key});

  @override
  ConsumerState<SetupAuthStep> createState() => _SetupAuthStepState();
}

class _SetupAuthStepState extends ConsumerState<SetupAuthStep>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  final _formKey = GlobalKey<FormState>();

  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  final _passwordConfirmController = TextEditingController();
  final _displayNameController = TextEditingController();

  String? _timezone;
  String? _birthday;
  bool _submitting = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _tabController.addListener(() {
      if (!_tabController.indexIsChanging) {
        setState(() => _errorMessage = null);
      }
    });
    _loadTimezone();
  }

  Future<void> _loadTimezone() async {
    final timezone = await FlutterTimezone.getLocalTimezone();
    if (mounted) {
      setState(() => _timezone = timezone.identifier);
    }
  }

  @override
  void dispose() {
    _tabController.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
    _passwordConfirmController.dispose();
    _displayNameController.dispose();
    super.dispose();
  }

  bool get _isRegister => _tabController.index == 0;

  Future<void> _pickBirthday() async {
    final now = DateTime.now();
    final picked = await showDatePicker(
      context: context,
      initialDate: DateTime(now.year - 20),
      firstDate: DateTime(1900),
      lastDate: now,
    );
    if (picked != null && mounted) {
      setState(() {
        _birthday =
            '${picked.year.toString().padLeft(4, '0')}-${picked.month.toString().padLeft(2, '0')}-${picked.day.toString().padLeft(2, '0')}';
      });
    }
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) {
      return;
    }

    final authApi = ref.read(authApiProvider);
    if (authApi == null) {
      setState(() => _errorMessage = '请先配置服务器地址');
      return;
    }

    setState(() {
      _submitting = true;
      _errorMessage = null;
    });

    try {
      final session = _isRegister
          ? await authApi.register(
              username: _usernameController.text.trim(),
              password: _passwordController.text,
              displayName: _displayNameController.text.trim(),
              birthday: _birthday,
              timezone: _timezone ?? 'UTC',
            )
          : await authApi.login(
              username: _usernameController.text.trim(),
              password: _passwordController.text,
            );

      await ref.read(authConfigProvider.notifier).setToken(session.token);
      if (mounted) {
        context.go('/home');
      }
    } on ApiException catch (error) {
      if (mounted) {
        setState(() => _errorMessage = _mapError(error.error));
      }
    } finally {
      if (mounted) {
        setState(() => _submitting = false);
      }
    }
  }

  String _mapError(ApiError error) {
    return switch (error.code) {
      'USERNAME_TAKEN' => '用户名已被占用',
      'INVALID_CREDENTIALS' => '用户名或密码错误',
      'BAD_REQUEST' => error.message,
      _ => error.message,
    };
  }

  @override
  Widget build(BuildContext context) {
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TabBar(
            controller: _tabController,
            onTap: (_) => setState(() => _errorMessage = null),
            tabs: const [
              Tab(text: '注册'),
              Tab(text: '登录'),
            ],
          ),
          const SizedBox(height: 24),
          TextFormField(
            controller: _usernameController,
            decoration: const InputDecoration(
              labelText: '用户名 *',
              border: OutlineInputBorder(),
            ),
            textInputAction: TextInputAction.next,
            validator: (value) {
              if (value == null || value.trim().isEmpty) {
                return '请输入用户名';
              }
              return null;
            },
          ),
          const SizedBox(height: 16),
          TextFormField(
            controller: _passwordController,
            decoration: const InputDecoration(
              labelText: '密码 *',
              border: OutlineInputBorder(),
            ),
            obscureText: true,
            textInputAction:
                _isRegister ? TextInputAction.next : TextInputAction.done,
            validator: (value) {
              if (value == null || value.length < 8) {
                return '密码至少 8 位';
              }
              return null;
            },
          ),
          if (_isRegister) ...[
            const SizedBox(height: 16),
            TextFormField(
              controller: _passwordConfirmController,
              decoration: const InputDecoration(
                labelText: '确认密码 *',
                border: OutlineInputBorder(),
              ),
              obscureText: true,
              validator: (value) {
                if (value != _passwordController.text) {
                  return '两次密码不一致';
                }
                return null;
              },
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _displayNameController,
              decoration: const InputDecoration(
                labelText: '称呼（可选）',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 16),
            ListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('生日（可选）'),
              subtitle: Text(_birthday ?? '未设置'),
              trailing: TextButton(
                onPressed: _pickBirthday,
                child: const Text('选择'),
              ),
            ),
            ListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('时区'),
              subtitle: Text(_timezone ?? '加载中…'),
            ),
          ],
          if (_errorMessage != null) ...[
            const SizedBox(height: 12),
            Text(
              _errorMessage!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
          const SizedBox(height: 24),
          FilledButton(
            onPressed: _submitting ? null : _submit,
            child: _submitting
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text(_isRegister ? '注册' : '登录'),
          ),
        ],
      ),
    );
  }
}
