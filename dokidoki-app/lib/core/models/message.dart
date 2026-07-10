class MessagePage {
  const MessagePage({required this.messages, required this.hasMore});

  final List<ChatMessage> messages;
  final bool hasMore;
}

class SentMessage {
  const SentMessage({
    required this.id,
    required this.turnId,
    required this.createdAt,
  });

  final String id;
  final String turnId;
  final String createdAt;

  factory SentMessage.fromJson(Map<String, dynamic> json) {
    return SentMessage(
      id: json['id'] as String,
      turnId: json['turn_id'] as String,
      createdAt: json['created_at'] as String,
    );
  }
}

class ChatMessage {
  const ChatMessage({
    required this.id,
    required this.conversationId,
    required this.role,
    required this.content,
    required this.contentType,
    required this.turnId,
    required this.seqInTurn,
    required this.createdAt,
    this.replyToId,
    this.readAt,
  });

  final String id;
  final String conversationId;
  final String role;
  final String content;
  final String contentType;
  final String? turnId;
  final int seqInTurn;
  final String createdAt;
  final String? replyToId;
  final String? readAt;

  bool get isText => contentType == 'text';
  bool get isImage => contentType == 'image';
  bool get isUser => role == 'user';
  bool get isCharacter => role == 'character';

  String get displayContent {
    if (isImage) {
      return content.isNotEmpty ? content : '[图片]';
    }
    return content;
  }

  factory ChatMessage.fromListJson(
    Map<String, dynamic> json, {
    required String conversationId,
  }) {
    final contentType = json['content_type'] as String;
    return ChatMessage(
      id: json['id'] as String,
      conversationId: conversationId,
      role: json['role'] as String,
      content: json['content'] as String? ?? '',
      contentType: contentType,
      turnId: json['turn_id'] as String?,
      seqInTurn: json['seq_in_turn'] as int? ?? 0,
      createdAt: json['created_at'] as String,
      replyToId: json['reply_to_id'] as String?,
      readAt: json['read_at'] as String?,
    );
  }

  factory ChatMessage.fromWsPayload(Map<String, dynamic> json) {
    return ChatMessage(
      id: json['id'] as String,
      conversationId: json['conversation_id'] as String,
      role: json['role'] as String,
      content: json['content'] as String? ?? '',
      contentType: json['content_type'] as String? ?? 'text',
      turnId: json['turn_id'] as String?,
      seqInTurn: json['seq_in_turn'] as int? ?? 0,
      createdAt: json['created_at'] as String,
      replyToId: json['reply_to_id'] as String?,
      readAt: json['read_at'] as String?,
    );
  }
}
