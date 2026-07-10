import 'package:flutter/material.dart';

import '../../../shared/widgets/placeholder_scaffold.dart';

class SetupPage extends StatelessWidget {
  const SetupPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const PlaceholderScaffold(
      title: 'Setup',
      subtitle: 'Server URL / Register / Login',
    );
  }
}
