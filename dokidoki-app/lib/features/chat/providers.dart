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
      if (event.type != 'message') {
        return;
      }
      final conversationId = event.payload['conversation_id'] as String?;
      if (conversationId != context.conversationId) {
        return;
      }
      _appendMessage(ChatMessage.fromWsPayload(event.payload));
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
    if (current == null || current.sending) {
      return;
    }

    final api = ref.read(messagesApiProvider);
    if (api == null) {
      return;
    }

    state = AsyncData(current.copyWith(sending: true));

    try {
      final sent = await api.sendText(context.conversationId, trimmed);
      final message = ChatMessage(
        id: sent.id,
        conversationId: context.conversationId,
        role: 'user',
        content: trimmed,
        contentType: 'text',
        turnId: sent.turnId,
        seqInTurn: 0,
        createdAt: sent.createdAt,
      );
      _appendMessage(message);

      final latest = state.value;
      if (latest != null) {
        state = AsyncData(latest.copyWith(sending: false));
      }
    } catch (error) {
      final latest = state.value;
      if (latest != null) {
        state = AsyncData(latest.copyWith(sending: false));
      }
      rethrow;
    }
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
}
