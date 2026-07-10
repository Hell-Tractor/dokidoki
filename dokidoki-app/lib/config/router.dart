import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/auth/auth_config.dart';
import '../core/auth/providers.dart';
import '../features/chat/chat_page.dart';
import '../features/home/home_page.dart';
import '../features/settings/character_settings_page.dart';
import '../features/settings/settings_page.dart';
import '../features/setup/setup_page.dart';
import '../features/setup/splash_page.dart';

final routerProvider = Provider<GoRouter>((ref) {
  final refreshNotifier = ValueNotifier<int>(0);

  ref.listen<AsyncValue<AuthConfig>>(authConfigProvider, (_, _) {
    refreshNotifier.value++;
  });

  ref.onDispose(refreshNotifier.dispose);

  return GoRouter(
    initialLocation: '/',
    refreshListenable: refreshNotifier,
    redirect: (context, state) {
      final config = ref.read(authConfigProvider).value;
      if (config == null) {
        return null;
      }

      final location = state.matchedLocation;
      final onSplash = location == '/';
      final onSetup = location == '/setup';

      if (!config.isAuthenticated) {
        if (!onSplash && !onSetup) {
          return config.hasServerUrl ? '/setup?step=2' : '/setup';
        }
      }

      return null;
    },
    routes: [
      GoRoute(
        path: '/',
        builder: (context, state) => const SplashPage(),
      ),
      GoRoute(
        path: '/setup',
        builder: (context, state) {
          final step = int.tryParse(state.uri.queryParameters['step'] ?? '') ?? 0;
          return SetupPage(initialStep: step == 2 ? 1 : 0);
        },
      ),
      GoRoute(
        path: '/home',
        builder: (context, state) => const HomePage(),
      ),
      GoRoute(
        path: '/settings',
        builder: (context, state) => const SettingsPage(),
      ),
      GoRoute(
        path: '/chat/:conversationId',
        builder: (context, state) => ChatPage(
          conversationId: state.pathParameters['conversationId']!,
        ),
        routes: [
          GoRoute(
            path: 'settings',
            builder: (context, state) => CharacterSettingsPage(
              conversationId: state.pathParameters['conversationId']!,
            ),
          ),
        ],
      ),
    ],
  );
});
