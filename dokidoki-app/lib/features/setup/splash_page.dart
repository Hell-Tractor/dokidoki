import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/api/providers.dart';
import '../../core/auth/providers.dart';
import '../../core/models/api_error.dart';

class SplashPage extends ConsumerStatefulWidget {
  const SplashPage({super.key});

  @override
  ConsumerState<SplashPage> createState() => _SplashPageState();
}

class _SplashPageState extends ConsumerState<SplashPage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _bootstrap());
  }

  Future<void> _bootstrap() async {
    final config = await ref.read(authConfigProvider.future);
    if (!mounted) {
      return;
    }

    if (!config.hasServerUrl) {
      context.go('/setup');
      return;
    }

    if (!config.hasToken) {
      context.go('/setup?step=2');
      return;
    }

    final authApi = ref.read(authApiProvider);
    if (authApi == null) {
      context.go('/setup');
      return;
    }

    try {
      await authApi.getMe();
      if (mounted) {
        context.go('/home');
      }
    } on ApiException {
      await ref.read(authConfigProvider.notifier).clearToken();
      if (mounted) {
        context.go('/setup?step=2');
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            FlutterLogo(size: 72),
            SizedBox(height: 24),
            CircularProgressIndicator(),
          ],
        ),
      ),
    );
  }
}
