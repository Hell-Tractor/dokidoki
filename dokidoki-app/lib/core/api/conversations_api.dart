import '../models/character.dart';
import '../models/conversation.dart';
import 'api_client.dart';

class ConversationsApi {
  ConversationsApi(this._client);

  final ApiClient _client;

  Future<List<ConversationListItem>> listConversations() {
    return _client.getData(
      '/conversations',
      parser: (json) => (json as List<dynamic>)
          .map((item) =>
              ConversationListItem.fromJson(item as Map<String, dynamic>))
          .toList(),
    );
  }

  Future<Conversation> createConversation(String characterId) {
    return _client.postData(
      '/conversations',
      data: {'character_id': characterId},
      parser: (json) =>
          Conversation.fromJson(json as Map<String, dynamic>),
    );
  }
}

class CharactersApi {
  CharactersApi(this._client);

  final ApiClient _client;

  Future<List<Character>> listCharacters() {
    return _client.getData(
      '/characters',
      parser: (json) => (json as List<dynamic>)
          .map((item) => Character.fromJson(item as Map<String, dynamic>))
          .toList(),
    );
  }
}
