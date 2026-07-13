import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/api/providers.dart';
import '../../core/models/api_error.dart';
import 'character_settings_args.dart';

class CharacterSettingsPage extends ConsumerStatefulWidget {
  const CharacterSettingsPage({
    super.key,
    required this.args,
  });

  final CharacterSettingsArgs args;

  @override
  ConsumerState<CharacterSettingsPage> createState() =>
      _CharacterSettingsPageState();
}

class _CharacterSettingsPageState extends ConsumerState<CharacterSettingsPage> {
  TimeOfDay? _dndStart;
  TimeOfDay? _dndEnd;
  bool _loading = true;
  String? _loadError;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    Future.microtask(_loadSettings);
  }

  String _formatTime(TimeOfDay time) {
    final hour = time.hour.toString().padLeft(2, '0');
    final minute = time.minute.toString().padLeft(2, '0');
    return '$hour:$minute';
  }

  TimeOfDay? _parseTime(String? value) {
    if (value == null || value.isEmpty) {
      return null;
    }
    final parts = value.split(':');
    if (parts.length != 2) {
      return null;
    }
    final hour = int.tryParse(parts[0]);
    final minute = int.tryParse(parts[1]);
    if (hour == null || minute == null) {
      return null;
    }
    return TimeOfDay(hour: hour, minute: minute);
  }

  String _displayTime(TimeOfDay? time) {
    if (time == null) {
      return '未设置';
    }
    return _formatTime(time);
  }

  Future<void> _loadSettings() async {
    final api = ref.read(charactersApiProvider);
    if (api == null) {
      if (mounted) {
        setState(() {
          _loading = false;
          _loadError = '未连接服务器';
        });
      }
      return;
    }

    try {
      final settings = await api.getCharacterSettings(widget.args.characterId);
      if (mounted) {
        setState(() {
          _dndStart = _parseTime(settings.dndStart);
          _dndEnd = _parseTime(settings.dndEnd);
          _loading = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _loading = false;
          _loadError = error.toString();
        });
      }
    }
  }

  Future<void> _pickTime({required bool isStart}) async {
    final initial =
        (isStart ? _dndStart : _dndEnd) ?? const TimeOfDay(hour: 23, minute: 0);
    final picked = await showTimePicker(
      context: context,
      initialTime: initial,
    );
    if (picked != null && mounted) {
      setState(() {
        if (isStart) {
          _dndStart = picked;
        } else {
          _dndEnd = picked;
        }
      });
    }
  }

  Future<void> _save() async {
    final api = ref.read(charactersApiProvider);
    if (api == null) {
      return;
    }

    setState(() => _saving = true);
    try {
      await api.updateCharacterSettings(
        widget.args.characterId,
        dndStart: _dndStart == null ? null : _formatTime(_dndStart!),
        dndEnd: _dndEnd == null ? null : _formatTime(_dndEnd!),
        clearDndStart: _dndStart == null,
        clearDndEnd: _dndEnd == null,
      );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('角色设置已保存')),
        );
        context.pop();
      }
    } on ApiException catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(error.error.message)),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('${widget.args.characterName} 的设置'),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _loadError != null
              ? Center(child: Text('加载失败：$_loadError'))
              : ListView(
                  padding: const EdgeInsets.all(16),
                  children: [
                    Text(
                      '勿扰时段',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            color: Theme.of(context).colorScheme.primary,
                          ),
                    ),
                    const SizedBox(height: 8),
                    const Text('按你在「设置」中配置的时区计算本地时间。跨午夜时开始时间晚于结束时间。'),
                    const SizedBox(height: 16),
                    ListTile(
                      contentPadding: EdgeInsets.zero,
                      title: const Text('勿扰开始'),
                      subtitle: Text(_displayTime(_dndStart)),
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (_dndStart != null)
                            TextButton(
                              onPressed: () => setState(() => _dndStart = null),
                              child: const Text('清除'),
                            ),
                          TextButton(
                            onPressed: () => _pickTime(isStart: true),
                            child: const Text('选择'),
                          ),
                        ],
                      ),
                    ),
                    ListTile(
                      contentPadding: EdgeInsets.zero,
                      title: const Text('勿扰结束'),
                      subtitle: Text(_displayTime(_dndEnd)),
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (_dndEnd != null)
                            TextButton(
                              onPressed: () => setState(() => _dndEnd = null),
                              child: const Text('清除'),
                            ),
                          TextButton(
                            onPressed: () => _pickTime(isStart: false),
                            child: const Text('选择'),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 24),
                    FilledButton(
                      onPressed: _saving ? null : _save,
                      child: _saving
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Text('保存'),
                    ),
                  ],
                ),
    );
  }
}
