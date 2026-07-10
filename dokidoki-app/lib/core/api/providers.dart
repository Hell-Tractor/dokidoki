import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../auth/providers.dart';
import 'api_client.dart';
import 'auth_api.dart';
import 'conversations_api.dart';

final apiClientProvider = Provider<ApiClient?>((ref) {
  final config = ref.watch(authConfigProvider).value;
  final baseUrl = config?.apiBaseUrl;
  if (baseUrl == null) {
    return null;
  }

  return ApiClient(
    baseUrl: baseUrl,
    getToken: () async => ref.read(authConfigProvider).value?.token,
    onUnauthorized: () {
      ref.read(authConfigProvider.notifier).clearToken();
    },
  );
});

final authApiProvider = Provider<AuthApi?>((ref) {
  final client = ref.watch(apiClientProvider);
  if (client == null) {
    return null;
  }
  return AuthApi(client);
});

final conversationsApiProvider = Provider<ConversationsApi?>((ref) {
  final client = ref.watch(apiClientProvider);
  if (client == null) {
    return null;
  }
  return ConversationsApi(client);
});

final charactersApiProvider = Provider<CharactersApi?>((ref) {
  final client = ref.watch(apiClientProvider);
  if (client == null) {
    return null;
  }
  return CharactersApi(client);
});
