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
  final String turnId;
  final int seqInTurn;
  final String createdAt;
  final String? replyToId;
  final String? readAt;

  factory ChatMessage.fromJson(Map<String, dynamic> json) {
    return ChatMessage(
      id: json['id'] as String,
      conversationId: json['conversation_id'] as String,
      role: json['role'] as String,
      content: json['content'] as String,
      contentType: json['content_type'] as String,
      turnId: json['turn_id'] as String,
      seqInTurn: json['seq_in_turn'] as int,
      createdAt: json['created_at'] as String,
      replyToId: json['reply_to_id'] as String?,
      readAt: json['read_at'] as String?,
    );
  }
}
