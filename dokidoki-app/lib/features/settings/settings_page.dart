import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_timezone/flutter_timezone.dart';
import 'package:go_router/go_router.dart';

import '../../core/api/auth_api.dart';
import '../../core/api/providers.dart';
import '../../core/auth/providers.dart';
import '../../core/models/api_error.dart';
import '../../core/models/user.dart';

class SettingsPage extends ConsumerStatefulWidget {
  const SettingsPage({super.key});

  @override
  ConsumerState<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  final _displayNameController = TextEditingController();
  final _timezoneController = TextEditingController();
  final _serverUrlController = TextEditingController();

  String? _birthday;
  int _maxProactivePerDay = 20;
  bool _profileLoaded = false;
  bool _savingProfile = false;
  bool _testingServer = false;
  bool _savingServer = false;
  bool _serverConnectionOk = false;
  String? _serverStatusMessage;

  @override
  void dispose() {
    _displayNameController.dispose();
    _timezoneController.dispose();
    _serverUrlController.dispose();
    super.dispose();
  }

  void _loadProfile(User user) {
    if (_profileLoaded) {
      return;
    }
    _displayNameController.text = user.displayName;
    _timezoneController.text = user.timezone;
    _birthday = user.birthday;
    _maxProactivePerDay = user.maxProactivePerDay;
    _profileLoaded = true;
  }

  void _loadServerUrl(String? serverUrl) {
    if (serverUrl != null && _serverUrlController.text.isEmpty) {
      _serverUrlController.text = serverUrl;
    }
  }

  Future<void> _pickBirthday() async {
    final now = DateTime.now();
    final initial = _birthday != null
        ? DateTime.tryParse(_birthday!) ?? DateTime(now.year - 20)
        : DateTime(now.year - 20);
    final picked = await showDatePicker(
      context: context,
      initialDate: initial,
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

  Future<void> _useDeviceTimezone() async {
    final timezone = await FlutterTimezone.getLocalTimezone();
    if (mounted) {
      setState(() => _timezoneController.text = timezone.identifier);
    }
  }

  Future<void> _saveProfile() async {
    final authApi = ref.read(authApiProvider);
    if (authApi == null) {
      return;
    }

    setState(() => _savingProfile = true);
    try {
      await authApi.patchMe(
        displayName: _displayNameController.text.trim(),
        birthday: _birthday,
        timezone: _timezoneController.text.trim(),
        maxProactivePerDay: _maxProactivePerDay,
      );
      ref.invalidate(currentUserProvider);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('档案已保存')),
        );
      }
    } on ApiException catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(error.error.message)),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _savingProfile = false);
      }
    }
  }

  Future<void> _testServer() async {
    final url = normalizeServerUrl(_serverUrlController.text);
    if (!isValidServerUrl(url)) {
      setState(() => _serverStatusMessage = '请输入有效的 http/https 地址');
      return;
    }

    setState(() {
      _testingServer = true;
      _serverConnectionOk = false;
      _serverStatusMessage = null;
    });

    try {
      await testServerConnection(url);
      if (mounted) {
        setState(() {
          _serverConnectionOk = true;
          _serverStatusMessage = '连接成功';
        });
      }
    } on ApiException catch (error) {
      if (mounted) {
        setState(() => _serverStatusMessage = error.error.message);
      }
    } finally {
      if (mounted) {
        setState(() => _testingServer = false);
      }
    }
  }

  Future<void> _saveServerUrl() async {
    final url = normalizeServerUrl(_serverUrlController.text);
    if (!isValidServerUrl(url)) {
      setState(() => _serverStatusMessage = '请输入有效的 http/https 地址');
      return;
    }
    if (!_serverConnectionOk) {
      setState(() => _serverStatusMessage = '请先测试连接');
      return;
    }

    setState(() => _savingServer = true);
    try {
      await ref.read(authConfigProvider.notifier).setServerUrl(url);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('服务器地址已保存')),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _savingServer = false);
      }
    }
  }

  Future<void> _logout() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('退出登录'),
        content: const Text('确定要退出当前账号吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('退出'),
          ),
        ],
      ),
    );

    if (confirmed != true || !mounted) {
      return;
    }

    await ref.read(authConfigProvider.notifier).clearToken();
    if (mounted) {
      context.go('/setup?step=2');
    }
  }

  @override
  Widget build(BuildContext context) {
    final userAsync = ref.watch(currentUserProvider);
    final serverUrl = ref.watch(authConfigProvider).value?.serverUrl;
    _loadServerUrl(serverUrl);
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Scaffold(
      appBar: AppBar(title: const Text('设置')),
      body: userAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => Center(child: Text('加载失败：$error')),
        data: (user) {
          if (user == null) {
            return const Center(child: Text('未登录'));
          }
          _loadProfile(user);

          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
            children: [
              _SettingsCard(
                title: '用户档案',
                subtitle: '角色如何称呼你，以及主动消息相关偏好',
                children: [
                  _ReadonlyRow(
                    label: '用户名',
                    value: user.username,
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _displayNameController,
                    textInputAction: TextInputAction.next,
                    decoration: const InputDecoration(
                      labelText: '称呼',
                      hintText: '角色对你的叫法',
                    ),
                  ),
                  const SizedBox(height: 16),
                  InkWell(
                    onTap: _pickBirthday,
                    borderRadius: BorderRadius.circular(16),
                    child: InputDecorator(
                      decoration: const InputDecoration(
                        labelText: '生日',
                        suffixIcon: Icon(Icons.calendar_today_outlined),
                      ),
                      child: Text(
                        _birthday ?? '未设置',
                        style: textTheme.bodyLarge?.copyWith(
                          color: _birthday == null
                              ? scheme.onSurfaceVariant
                              : scheme.onSurface,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _timezoneController,
                    decoration: InputDecoration(
                      labelText: '时区',
                      hintText: '例如 Asia/Shanghai',
                      suffixIcon: IconButton(
                        icon: const Icon(Icons.my_location_outlined),
                        tooltip: '使用设备时区',
                        onPressed: _useDeviceTimezone,
                      ),
                    ),
                  ),
                  const SizedBox(height: 20),
                  Text(
                    '每日主动消息上限',
                    style: textTheme.labelLarge?.copyWith(
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      IconButton.filledTonal(
                        onPressed: _maxProactivePerDay > 0
                            ? () => setState(() => _maxProactivePerDay--)
                            : null,
                        icon: const Icon(Icons.remove),
                      ),
                      Expanded(
                        child: Text(
                          '$_maxProactivePerDay',
                          textAlign: TextAlign.center,
                          style: textTheme.titleLarge,
                        ),
                      ),
                      IconButton.filledTonal(
                        onPressed: _maxProactivePerDay < 100
                            ? () => setState(() => _maxProactivePerDay++)
                            : null,
                        icon: const Icon(Icons.add),
                      ),
                    ],
                  ),
                  const SizedBox(height: 20),
                  SizedBox(
                    width: double.infinity,
                    child: FilledButton(
                      onPressed: _savingProfile ? null : _saveProfile,
                      child: _savingProfile
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Text('保存档案'),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              _SettingsCard(
                title: '连接',
                subtitle: '后端地址，修改后需先测试再保存',
                children: [
                  TextField(
                    controller: _serverUrlController,
                    decoration: const InputDecoration(
                      labelText: '服务器地址',
                      hintText: 'https://example.com',
                    ),
                    keyboardType: TextInputType.url,
                    onChanged: (_) {
                      if (_serverConnectionOk || _serverStatusMessage != null) {
                        setState(() {
                          _serverConnectionOk = false;
                          _serverStatusMessage = null;
                        });
                      }
                    },
                  ),
                  if (_serverStatusMessage != null) ...[
                    const SizedBox(height: 12),
                    _StatusBanner(
                      message: _serverStatusMessage!,
                      ok: _serverConnectionOk,
                    ),
                  ],
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Expanded(
                        child: OutlinedButton(
                          onPressed: _testingServer ? null : _testServer,
                          child: _testingServer
                              ? const SizedBox(
                                  width: 18,
                                  height: 18,
                                  child:
                                      CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Text('测试连接'),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: FilledButton.tonal(
                          onPressed: _savingServer ? null : _saveServerUrl,
                          child: _savingServer
                              ? const SizedBox(
                                  width: 18,
                                  height: 18,
                                  child:
                                      CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Text('保存地址'),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 24),
              SizedBox(
                width: double.infinity,
                child: OutlinedButton(
                  onPressed: _logout,
                  style: OutlinedButton.styleFrom(
                    foregroundColor: scheme.error,
                    side: BorderSide(color: scheme.error.withValues(alpha: 0.45)),
                    padding: const EdgeInsets.symmetric(vertical: 14),
                  ),
                  child: const Text('退出登录'),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _SettingsCard extends StatelessWidget {
  const _SettingsCard({
    required this.title,
    required this.subtitle,
    required this.children,
  });

  final String title;
  final String subtitle;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              title,
              style: textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.w600,
                color: scheme.primary,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              subtitle,
              style: textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 16),
            ...children,
          ],
        ),
      ),
    );
  }
}

class _ReadonlyRow extends StatelessWidget {
  const _ReadonlyRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      decoration: BoxDecoration(
        color: scheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: scheme.outlineVariant),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: textTheme.labelMedium?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          Text(value, style: textTheme.bodyLarge),
        ],
      ),
    );
  }
}

class _StatusBanner extends StatelessWidget {
  const _StatusBanner({required this.message, required this.ok});

  final String message;
  final bool ok;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final color = ok ? scheme.primary : scheme.error;
    final bg = ok
        ? scheme.primaryContainer.withValues(alpha: 0.55)
        : scheme.errorContainer.withValues(alpha: 0.55);

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          Icon(
            ok ? Icons.check_circle_outline : Icons.error_outline,
            size: 18,
            color: color,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: color,
                  ),
            ),
          ),
        ],
      ),
    );
  }
}
