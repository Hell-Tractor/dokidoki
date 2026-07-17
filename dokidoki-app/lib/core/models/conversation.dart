class LastMessage {
  const LastMessage({
    required this.content,
    required this.createdAt,
    required this.role,
  });

  final String content;
  final String createdAt;
  final String role;

  factory LastMessage.fromJson(Map<String, dynamic> json) {
    return LastMessage(
      content: json['content'] as String,
      createdAt: json['created_at'] as String,
      role: json['role'] as String,
    );
  }
}

class ConversationListItem {
  const ConversationListItem({
    required this.id,
    required this.characterId,
    required this.characterName,
    required this.status,
    this.lastMessage,
    this.currentActivity,
  });

  final String id;
  final String characterId;
  final String characterName;
  final String status;
  final LastMessage? lastMessage;
  final String? currentActivity;

  ConversationListItem copyWith({
    String? status,
    LastMessage? lastMessage,
    String? currentActivity,
  }) {
    return ConversationListItem(
      id: id,
      characterId: characterId,
      characterName: characterName,
      status: status ?? this.status,
      lastMessage: lastMessage ?? this.lastMessage,
      currentActivity: currentActivity ?? this.currentActivity,
    );
  }

  factory ConversationListItem.fromJson(Map<String, dynamic> json) {
    return ConversationListItem(
      id: json['id'] as String,
      characterId: json['character_id'] as String,
      characterName: json['character_name'] as String,
      status: json['status'] as String,
      lastMessage: json['last_message'] == null
          ? null
          : LastMessage.fromJson(json['last_message'] as Map<String, dynamic>),
      currentActivity: json['current_activity'] as String?,
    );
  }
}

class Conversation {
  const Conversation({
    required this.id,
    required this.characterId,
    required this.status,
    required this.firstContactDone,
  });

  final String id;
  final String characterId;
  final String status;
  final bool firstContactDone;

  factory Conversation.fromJson(Map<String, dynamic> json) {
    return Conversation(
      id: json['id'] as String,
      characterId: json['character_id'] as String,
      status: json['status'] as String,
      firstContactDone: json['first_contact_done'] as bool,
    );
  }
}
