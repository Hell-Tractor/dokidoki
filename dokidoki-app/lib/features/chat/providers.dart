import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/providers.dart';
import '../../core/models/message.dart';
import '../../core/ws/providers.dart';
import '../home/providers.dart';
import 'chat_state.dart';

final chatProvider =
    AsyncNotifierProvider.family<ChatNotifier, ChatState, ChatContext>(
  ChatNotifier.new,
);

class ChatNotifier extends AsyncNotifier<ChatState> {
  ChatNotifier(this.context);

  final ChatContext context;
  StreamSubscription<dynamic>? _wsSubscription;

  @override
  Future<ChatState> build() async {
    final api = ref.read(messagesApiProvider);
    if (api == null) {
      throw StateError('API client unavailable');
    }

    final ws = ref.read(wsClientProvider);
    ws.subscribe(context.conversationId);

    _wsSubscription?.cancel();
    _wsSubscription = ws.events.listen((event) {
      switch (event.type) {
        case 'message':
          final conversationId = event.payload['conversation_id'] as String?;
          if (conversationId != context.conversationId) {
            return;
          }
          _appendMessage(ChatMessage.fromWsPayload(event.payload));
        case 'character_typing':
          final conversationId = event.payload['conversation_id'] as String?;
          if (conversationId != context.conversationId) {
            return;
          }
          _setCharacterTyping(event.payload['active'] as bool? ?? false);
        case 'turn_cancelled':
          final conversationId = event.payload['conversation_id'] as String?;
          if (conversationId != context.conversationId) {
            return;
          }
          final turnId = event.payload['turn_id'] as String?;
          if (turnId != null) {
            _removeCharacterMessagesForTurn(turnId);
          }
        case 'message_read':
          final conversationId = event.payload['conversation_id'] as String?;
          if (conversationId != context.conversationId) {
            return;
          }
          final readAt = event.payload['read_at'] as String?;
          final messageIds = (event.payload['message_ids'] as List<dynamic>?)
                  ?.map((id) => id as String)
                  .toList() ??
              const <String>[];
          _markMessagesRead(messageIds, readAt);
        default:
          break;
      }
    });
    ref.onDispose(() => _wsSubscription?.cancel());

    final character = _resolveCharacter(context);
    final page = await api.listMessages(context.conversationId);

    return ChatState(
      messages: page.messages,
      hasMore: page.hasMore,
      characterId: character.characterId,
      characterName: character.characterName,
    );
  }

  ChatContext _resolveCharacter(ChatContext chatContext) {
    if (chatContext.characterName != null) {
      return chatContext;
    }

    final conversations = ref.read(conversationsProvider).value;
    final match = conversations?.where(
      (item) => item.id == chatContext.conversationId,
    );
    if (match == null || match.isEmpty) {
      return chatContext;
    }

    final item = match.first;
    return ChatContext(
      conversationId: chatContext.conversationId,
      characterId: item.characterId,
      characterName: item.characterName,
    );
  }

  Future<void> loadMore() async {
    final current = state.value;
    if (current == null || !current.hasMore || current.loadingMore) {
      return;
    }
    if (current.messages.isEmpty) {
      return;
    }

    final api = ref.read(messagesApiProvider);
    if (api == null) {
      return;
    }

    state = AsyncData(current.copyWith(loadingMore: true));

    try {
      final page = await api.listMessages(
        context.conversationId,
        before: current.messages.first.id,
      );
      final existingIds = current.messages.map((m) => m.id).toSet();
      final older = page.messages.where((m) => !existingIds.contains(m.id));
      final currentState = state.value;
      if (currentState == null) {
        return;
      }

      state = AsyncData(
        currentState.copyWith(
          messages: [...older, ...currentState.messages],
          hasMore: page.hasMore,
          loadingMore: false,
        ),
      );
    } catch (_) {
      final latest = state.value;
      if (latest != null) {
        state = AsyncData(latest.copyWith(loadingMore: false));
      }
    }
  }

  Future<void> sendText(String content) async {
    final trimmed = content.trim();
    if (trimmed.isEmpty) {
      return;
    }

    final current = state.value;
    if (current == null) {
      return;
    }

    final localId =
        'local_${DateTime.now().microsecondsSinceEpoch}_${current.messages.length}';
    final optimistic = ChatMessage(
      id: localId,
      conversationId: context.conversationId,
      role: 'user',
      content: trimmed,
      contentType: 'text',
      turnId: null,
      seqInTurn: 0,
      createdAt: DateTime.now().toUtc().toIso8601String(),
      sendStatus: MessageSendStatus.sending,
    );
    _appendMessage(optimistic);
    await _dispatchSend(localId, trimmed);
  }

  Future<void> retryFailed(String messageId) async {
    final current = state.value;
    if (current == null) {
      return;
    }

    final index = current.messages.indexWhere((m) => m.id == messageId);
    if (index < 0) {
      return;
    }

    final message = current.messages[index];
    if (!message.isFailed || !message.isUser || !message.isText) {
      return;
    }

    _replaceMessage(
      messageId,
      message.copyWith(sendStatus: MessageSendStatus.sending),
    );
    await _dispatchSend(messageId, message.content);
  }

  Future<void> _dispatchSend(String localId, String content) async {
    final api = ref.read(messagesApiProvider);
    if (api == null) {
      _replaceMessage(
        localId,
        _messageById(localId)?.copyWith(sendStatus: MessageSendStatus.failed),
      );
      return;
    }

    try {
      final sent = await api.sendText(context.conversationId, content);
      final pending = _messageById(localId);
      if (pending == null) {
        return;
      }

      _replaceMessage(
        localId,
        pending.copyWith(
          id: sent.id,
          turnId: sent.turnId,
          createdAt: sent.createdAt,
          sendStatus: MessageSendStatus.sent,
        ),
      );
    } catch (_) {
      final pending = _messageById(localId);
      if (pending != null) {
        _replaceMessage(
          localId,
          pending.copyWith(sendStatus: MessageSendStatus.failed),
        );
      }
    }
  }

  ChatMessage? _messageById(String id) {
    final current = state.value;
    if (current == null) {
      return null;
    }
    for (final message in current.messages) {
      if (message.id == id) {
        return message;
      }
    }
    return null;
  }

  void _replaceMessage(String id, ChatMessage? next) {
    if (next == null) {
      return;
    }
    final current = state.value;
    if (current == null) {
      return;
    }

    final messages = [
      for (final message in current.messages)
        if (message.id == id) next else message,
    ];
    state = AsyncData(current.copyWith(messages: messages));
  }

  void _appendMessage(ChatMessage message) {
    final current = state.value;
    if (current == null) {
      return;
    }
    if (current.messages.any((item) => item.id == message.id)) {
      return;
    }

    state = AsyncData(
      current.copyWith(messages: [...current.messages, message]),
    );
  }

  void _setCharacterTyping(bool active) {
    final current = state.value;
    if (current == null) {
      return;
    }
    if (current.isCharacterTyping == active) {
      return;
    }
    state = AsyncData(current.copyWith(isCharacterTyping: active));
  }

  void _removeCharacterMessagesForTurn(String turnId) {
    final current = state.value;
    if (current == null) {
      return;
    }
    final filtered = current.messages
        .where((m) => !(m.isCharacter && m.turnId == turnId))
        .toList();
    if (filtered.length == current.messages.length) {
      return;
    }
    state = AsyncData(current.copyWith(messages: filtered));
  }

  void _markMessagesRead(List<String> messageIds, String? readAt) {
    if (readAt == null || messageIds.isEmpty) {
      return;
    }

    final current = state.value;
    if (current == null) {
      return;
    }

    final idSet = messageIds.toSet();
    final updated = current.messages
        .map(
          (message) => idSet.contains(message.id)
              ? message.copyWith(readAt: readAt)
              : message,
        )
        .toList();

    state = AsyncData(current.copyWith(messages: updated));
  }
}
