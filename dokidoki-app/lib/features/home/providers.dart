import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/providers.dart';
import '../../core/models/character.dart';
import '../../core/models/conversation.dart';

final conversationsProvider =
    AsyncNotifierProvider<ConversationsNotifier, List<ConversationListItem>>(
  ConversationsNotifier.new,
);

class ConversationsNotifier extends AsyncNotifier<List<ConversationListItem>> {
  @override
  Future<List<ConversationListItem>> build() async {
    final api = ref.watch(conversationsApiProvider);
    if (api == null) {
      return [];
    }
    return api.listConversations();
  }

  Future<void> refresh() async {
    final api = ref.read(conversationsApiProvider);
    if (api == null) {
      state = const AsyncData([]);
      return;
    }

    state = await AsyncValue.guard(api.listConversations);
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
