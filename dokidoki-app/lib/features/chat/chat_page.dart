import 'package:flutter/material.dart';

import '../../shared/widgets/placeholder_scaffold.dart';

class ChatPage extends StatelessWidget {
  const ChatPage({super.key, required this.conversationId});

  final String conversationId;

  @override
  Widget build(BuildContext context) {
    return PlaceholderScaffold(
      title: 'Chat',
      subtitle: conversationId,
    );
  }
}
