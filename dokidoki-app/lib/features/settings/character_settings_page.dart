import 'package:flutter/material.dart';

import '../../shared/widgets/placeholder_scaffold.dart';

class CharacterSettingsPage extends StatelessWidget {
  const CharacterSettingsPage({
    super.key,
    required this.conversationId,
  });

  final String conversationId;

  @override
  Widget build(BuildContext context) {
    return PlaceholderScaffold(
      title: 'Character Settings',
      subtitle: conversationId,
    );
  }
}
