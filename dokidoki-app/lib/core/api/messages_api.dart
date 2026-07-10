import '../models/message.dart';
import 'api_client.dart';

class MessagesApi {
  MessagesApi(this._client);

  final ApiClient _client;

  Future<MessagePage> listMessages(
    String conversationId, {
    String? before,
    int limit = 50,
  }) {
    return _client.getData(
      '/conversations/$conversationId/messages',
      queryParameters: {
        if (before != null) 'before': before,
        'limit': limit,
      },
      parser: (json) {
        final body = json as Map<String, dynamic>;
        final items = body['messages'] as List<dynamic>;
        return MessagePage(
          messages: items
              .map(
                (item) => ChatMessage.fromListJson(
                  item as Map<String, dynamic>,
                  conversationId: conversationId,
                ),
              )
              .toList(),
          hasMore: body['has_more'] as bool? ?? false,
        );
      },
    );
  }

  Future<SentMessage> sendText(String conversationId, String content) {
    return _client.postData(
      '/conversations/$conversationId/messages',
      data: {'content': content},
      parser: (json) =>
          SentMessage.fromJson(json as Map<String, dynamic>),
    );
  }
}
