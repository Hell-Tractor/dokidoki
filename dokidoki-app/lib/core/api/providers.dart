import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../auth/providers.dart';
import 'api_client.dart';

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
