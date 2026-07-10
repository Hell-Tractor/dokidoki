import 'package:flutter/material.dart';

import '../../shared/widgets/placeholder_scaffold.dart';

class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    return const PlaceholderScaffold(
      title: 'Home',
      subtitle: 'Conversation list',
    );
  }
}
