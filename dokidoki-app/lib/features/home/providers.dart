import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/providers.dart';
import '../../core/models/character.dart';
import '../../core/models/conversation.dart';
import '../../core/models/message.dart';
import '../../core/ws/providers.dart';
import '../../core/ws/ws_client.dart';

final conversationsProvider =
    AsyncNotifierProvider<ConversationsNotifier, List<ConversationListItem>>(
  ConversationsNotifier.new,
);

class ConversationsNotifier extends AsyncNotifier<List<ConversationListItem>> {
  StreamSubscription<WsEvent>? _wsEvents;
  StreamSubscription<WsConnectionState>? _wsState;
  final Set<String> _subscribedIds = <String>{};

  @override
  Future<List<ConversationListItem>> build() async {
    _attachWsListeners();

    final api = ref.watch(conversationsApiProvider);
    if (api == null) {
      return [];
    }
    final list = await api.listConversations();
    _syncSubscriptions(list.map((item) => item.id));
    return list;
  }

  void _attachWsListeners() {
    final ws = ref.read(wsClientProvider);

    _wsEvents?.cancel();
    _wsEvents = ws.events.listen(_onWsEvent);

    _wsState?.cancel();
    _wsState = ws.connectionState.listen((connectionState) {
      if (connectionState != WsConnectionState.connected) {
        return;
      }
      // 重连后服务端订阅清空，需要重新 subscribe。
      _subscribedIds.clear();
      final list = state.valueOrNull ?? const <ConversationListItem>[];
      _syncSubscriptions(list.map((item) => item.id));
    });

    ref.onDispose(() {
      _wsEvents?.cancel();
      _wsState?.cancel();
      _subscribedIds.clear();
    });
  }

  void _syncSubscriptions(Iterable<String> conversationIds) {
    final ws = ref.read(wsClientProvider);
    for (final id in conversationIds) {
      if (_subscribedIds.add(id)) {
        ws.subscribe(id);
      }
    }
  }

  void _onWsEvent(WsEvent event) {
    if (event.type != 'message') {
      return;
    }
    final conversationId = event.payload['conversation_id'] as String?;
    if (conversationId == null) {
      return;
    }

    final message = ChatMessage.fromWsPayload(event.payload);
    _applyIncomingMessage(message);
  }

  void _applyIncomingMessage(ChatMessage message) {
    final current = state.valueOrNull;
    if (current == null || current.isEmpty) {
      return;
    }

    final index = current.indexWhere((item) => item.id == message.conversationId);
    if (index < 0) {
      return;
    }

    final updated = current[index].copyWith(
      lastMessage: LastMessage(
        content: message.displayContent,
        createdAt: message.createdAt,
        role: message.role,
      ),
    );

    final next = [...current];
    next.removeAt(index);
    next.insert(0, updated);
    state = AsyncData(next);
  }

  Future<void> refresh() async {
    final api = ref.read(conversationsApiProvider);
    if (api == null) {
      state = const AsyncData([]);
      return;
    }

    state = await AsyncValue.guard(api.listConversations);
    final list = state.valueOrNull ?? const <ConversationListItem>[];
    _syncSubscriptions(list.map((item) => item.id));
  }

  Future<Conversation> createConversation(String characterId) async {
    final api = ref.read(conversationsApiProvider);
    if (api == null) {
      throw StateError('API client unavailable');
    }

    final conversation = await api.createConversation(characterId);
    await refresh();
    return conversation;
  }
}

final charactersProvider = FutureProvider<List<Character>>((ref) async {
  final api = ref.watch(charactersApiProvider);
  if (api == null) {
    return [];
  }
  return api.listCharacters();
});
