import '../../core/models/conversation.dart';
import '../../core/models/message.dart';

class ChatContext {
  const ChatContext({
    required this.conversationId,
    this.characterId,
    this.characterName,
  });

  final String conversationId;
  final String? characterId;
  final String? characterName;

  factory ChatContext.fromConversation(ConversationListItem item) {
    return ChatContext(
      conversationId: item.id,
      characterId: item.characterId,
      characterName: item.characterName,
    );
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is ChatContext && conversationId == other.conversationId;
  }

  @override
  int get hashCode => conversationId.hashCode;
}

class ChatState {
  const ChatState({
    required this.messages,
    required this.hasMore,
    this.loadingMore = false,
    this.isCharacterTyping = false,
    this.characterId,
    this.characterName,
  });

  final List<ChatMessage> messages;
  final bool hasMore;
  final bool loadingMore;
  final bool isCharacterTyping;
  final String? characterId;
  final String? characterName;

  ChatState copyWith({
    List<ChatMessage>? messages,
    bool? hasMore,
    bool? loadingMore,
    bool? isCharacterTyping,
    String? characterId,
    String? characterName,
  }) {
    return ChatState(
      messages: messages ?? this.messages,
      hasMore: hasMore ?? this.hasMore,
      loadingMore: loadingMore ?? this.loadingMore,
      isCharacterTyping: isCharacterTyping ?? this.isCharacterTyping,
      characterId: characterId ?? this.characterId,
      characterName: characterName ?? this.characterName,
    );
  }
}
