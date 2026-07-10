import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../auth/providers.dart';
import 'ws_client.dart';

final wsClientProvider = Provider<WsClient>((ref) {
  final client = WsClient(
    getToken: () async => ref.read(authConfigProvider).value?.token,
    getWsUrl: () => ref.read(authConfigProvider).value?.wsUrl,
  );

  ref.listen(authConfigProvider, (previous, next) {
    final config = next.value;
    if (config != null && config.isAuthenticated) {
      client.connect();
    } else {
      client.disconnect();
    }
  });

  final config = ref.read(authConfigProvider).value;
  if (config != null && config.isAuthenticated) {
    client.connect();
  }

  ref.onDispose(client.dispose);
  return client;
});
