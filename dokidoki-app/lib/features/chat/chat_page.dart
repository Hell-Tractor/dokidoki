import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/api/providers.dart';
import '../../core/auth/providers.dart';
import '../../core/models/conversation.dart';
import '../../core/utils/url_utils.dart';
import '../../shared/widgets/character_avatar.dart';
import '../settings/character_settings_args.dart';
import 'chat_state.dart';
import 'providers.dart';
import 'widgets/chat_input_bar.dart';
import 'widgets/message_bubble.dart';

class ChatPage extends ConsumerStatefulWidget {
  const ChatPage({
    super.key,
    required this.conversationId,
    this.conversation,
  });

  final String conversationId;
  final ConversationListItem? conversation;

  @override
  ConsumerState<ChatPage> createState() => _ChatPageState();
}

class _ChatPageState extends ConsumerState<ChatPage> {
  final _inputController = TextEditingController();
  final _inputFocusNode = FocusNode();
  final _scrollController = ScrollController();

  ChatContext get _chatContext {
    if (widget.conversation != null) {
      return ChatContext.fromConversation(widget.conversation!);
    }
    return ChatContext(conversationId: widget.conversationId);
  }

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
  }

  @override
  void dispose() {
    _scrollController
      ..removeListener(_onScroll)
      ..dispose();
    _inputController.dispose();
    _inputFocusNode.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) {
      return;
    }

    final position = _scrollController.position;
    if (position.pixels >= position.maxScrollExtent - 80) {
      ref.read(chatProvider(_chatContext).notifier).loadMore();
    }
  }

  void _sendMessage() {
    final text = _inputController.text;
    if (text.trim().isEmpty) {
      return;
    }

    _inputController.clear();
    ref.read(chatProvider(_chatContext).notifier).sendText(text);
    // onSubmitted 会清掉焦点，下一帧再要回来以支持连续输入。
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _inputFocusNode.requestFocus();
      }
    });
  }

  void _retryMessage(String messageId) {
    ref.read(chatProvider(_chatContext).notifier).retryFailed(messageId);
  }

  Future<void> _scrollToBottom() async {
    if (!_scrollController.hasClients) {
      return;
    }
    await Future<void>.delayed(const Duration(milliseconds: 50));
    if (mounted && _scrollController.hasClients) {
      _scrollController.animateTo(
        0,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final chatAsync = ref.watch(chatProvider(_chatContext));
    final userAsync = ref.watch(currentUserProvider);
    final serverUrl = ref.watch(authConfigProvider).value?.serverUrl;

    ref.listen(chatProvider(_chatContext), (previous, next) {
      final prevCount = previous?.value?.messages.length ?? 0;
      final nextCount = next.value?.messages.length ?? 0;
      if (nextCount > prevCount) {
        _scrollToBottom();
      }
    });

    final userName = userAsync.value?.displayName ?? '我';

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.pop(),
        ),
        title: chatAsync.maybeWhen(
          data: (chat) {
            final characterName = chat.characterName ?? '聊天';
            final avatarUrl = chat.characterId != null && serverUrl != null
                ? resolveServerResource(
                    serverUrl,
                    '/api/v1/characters/${chat.characterId}/avatar',
                  )
                : null;

            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    CharacterAvatar(
                      name: characterName,
                      imageUrl: avatarUrl,
                      radius: 16,
                    ),
                    const SizedBox(width: 10),
                    Text(characterName),
                  ],
                ),
                if (chat.isCharacterTyping)
                  Text(
                    '对方正在输入…',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Colors.white.withValues(alpha: 0.85),
                        ),
                  ),
              ],
            );
          },
          orElse: () => const Text('聊天'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.more_vert),
            onPressed: () {
              final chat = ref.read(chatProvider(_chatContext)).value;
              final characterId =
                  chat?.characterId ?? widget.conversation?.characterId;
              final characterName = chat?.characterName ??
                  widget.conversation?.characterName ??
                  '角色';
              if (characterId == null) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('无法获取角色信息')),
                );
                return;
              }
              context.push(
                '/chat/${widget.conversationId}/settings',
                extra: CharacterSettingsArgs(
                  characterId: characterId,
                  characterName: characterName,
                ),
              );
            },
          ),
        ],
      ),
      body: Column(
        children: [
          Expanded(
            child: chatAsync.when(
              loading: () =>
                  const Center(child: CircularProgressIndicator()),
              error: (error, _) => Center(child: Text('加载失败：$error')),
              data: (chat) {
                if (chat.messages.isEmpty) {
                  return const Center(child: Text('暂无消息，发一条试试吧'));
                }

                return Stack(
                  children: [
                    ListView.builder(
                      controller: _scrollController,
                      reverse: true,
                      padding: const EdgeInsets.symmetric(vertical: 8),
                      itemCount: chat.messages.length,
                      itemBuilder: (context, index) {
                        final dataIndex = chat.messages.length - 1 - index;
                        final message = chat.messages[dataIndex];
                        final avatarUrl =
                            chat.characterId != null && serverUrl != null
                                ? resolveServerResource(
                                    serverUrl,
                                    '/api/v1/characters/${chat.characterId}/avatar',
                                  )
                                : null;

                        return MessageBubble(
                          message: message,
                          showAvatar: shouldShowAvatar(
                            chat.messages,
                            dataIndex,
                          ),
                          characterName: chat.characterName ?? '角色',
                          userDisplayName: userName,
                          characterAvatarUrl: avatarUrl,
                          onRetry: message.isFailed
                              ? () => _retryMessage(message.id)
                              : null,
                        );
                      },
                    ),
                    if (chat.loadingMore)
                      const Positioned(
                        top: 8,
                        left: 0,
                        right: 0,
                        child: Center(
                          child: SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                        ),
                      ),
                  ],
                );
              },
            ),
          ),
          ChatInputBar(
            controller: _inputController,
            focusNode: _inputFocusNode,
            onSend: _sendMessage,
          ),
        ],
      ),
    );
  }
}
