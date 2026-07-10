import 'package:flutter/material.dart';

import '../../shared/widgets/placeholder_scaffold.dart';

class SettingsPage extends StatelessWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const PlaceholderScaffold(
      title: 'Settings',
      subtitle: 'Profile & server URL',
    );
  }
}
