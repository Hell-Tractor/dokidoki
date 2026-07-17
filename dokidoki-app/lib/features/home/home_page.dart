import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/models/character.dart';
import '../../core/models/conversation.dart';
import '../../core/ws/providers.dart';
import 'character_picker_sheet.dart';
import 'conversation_tile.dart';
import 'providers.dart';

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(wsClientProvider).connect();
    });
  }

  Future<void> _refreshConversations() {
    return ref.read(conversationsProvider.notifier).refresh();
  }

  /// 会话列表走 HTTP，不订阅 WS；从聊天返回后主动拉一次摘要。
  Future<void> _openChat(ConversationListItem item) async {
    await context.push('/chat/${item.id}', extra: item);
    if (mounted) {
      await _refreshConversations();
    }
  }

  Future<void> _openCharacterPicker() async {
    final character = await showModalBottomSheet<Character>(
      context: context,
      isScrollControlled: true,
      builder: (context) => const CharacterPickerSheet(),
    );
    if (character == null || !mounted) {
      return;
    }

    try {
      final conversation = await ref
          .read(conversationsProvider.notifier)
          .createConversation(character.id);
      if (mounted) {
        final item = ConversationListItem(
          id: conversation.id,
          characterId: conversation.characterId,
          characterName: character.name,
          status: conversation.status,
        );
        await _openChat(item);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('创建会话失败：$error')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final conversationsAsync = ref.watch(conversationsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Dokidoki'),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings_outlined),
            tooltip: '设置',
            onPressed: () async {
              await context.push('/settings');
              if (mounted) {
                await _refreshConversations();
              }
            },
          ),
        ],
      ),
      body: conversationsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => _ErrorState(
          message: '$error',
          onRetry: () => ref.invalidate(conversationsProvider),
        ),
        data: (conversations) {
          if (conversations.isEmpty) {
            return _EmptyState(onPickCharacter: _openCharacterPicker);
          }

          return RefreshIndicator(
            onRefresh: _refreshConversations,
            child: ListView.separated(
              itemCount: conversations.length,
              separatorBuilder: (_, _) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final item = conversations[index];
                return ConversationTile(
                  item: item,
                  onTap: () => _openChat(item),
                );
              },
            ),
          );
        },
      ),
      floatingActionButton: conversationsAsync.maybeWhen(
        data: (conversations) => conversations.isNotEmpty
            ? FloatingActionButton(
                onPressed: _openCharacterPicker,
                child: const Icon(Icons.add),
              )
            : null,
        orElse: () => null,
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.onPickCharacter});

  final VoidCallback onPickCharacter;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.chat_bubble_outline,
              size: 64,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: 16),
            Text(
              '暂无会话',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              '选择一个角色开始聊天',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: onPickCharacter,
              child: const Text('选择角色'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ErrorState extends StatelessWidget {
  const _ErrorState({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton(
              onPressed: onRetry,
              child: const Text('重试'),
            ),
          ],
        ),
      ),
    );
  }
}
