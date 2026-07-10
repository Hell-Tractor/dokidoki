import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'config/router.dart';
import 'config/theme.dart';

class DokidokiApp extends ConsumerWidget {
  const DokidokiApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider);

    return MaterialApp.router(
      title: 'Dokidoki',
      theme: AppTheme.light(),
      routerConfig: router,
    );
  }
}
