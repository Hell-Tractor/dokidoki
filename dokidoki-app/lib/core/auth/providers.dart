import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'auth_config.dart';
import 'auth_repository.dart';

final sharedPreferencesProvider = Provider<SharedPreferences>((ref) {
  throw UnimplementedError('sharedPreferencesProvider must be overridden');
});

final secureStorageProvider = Provider<FlutterSecureStorage>(
  (ref) => const FlutterSecureStorage(),
);

final authRepositoryProvider = Provider<AuthRepository>((ref) {
  return AuthRepository(
    secureStorage: ref.watch(secureStorageProvider),
    preferences: ref.watch(sharedPreferencesProvider),
  );
});

final authConfigProvider =
    AsyncNotifierProvider<AuthConfigNotifier, AuthConfig>(
  AuthConfigNotifier.new,
);

class AuthConfigNotifier extends AsyncNotifier<AuthConfig> {
  @override
  Future<AuthConfig> build() {
    return ref.read(authRepositoryProvider).load();
  }

  Future<void> setServerUrl(String serverUrl) async {
    final repo = ref.read(authRepositoryProvider);
    await repo.saveServerUrl(serverUrl);
    final current = state.value ?? AuthConfig.empty;
    state = AsyncData(current.copyWith(serverUrl: serverUrl));
  }

  Future<void> setToken(String token) async {
    final repo = ref.read(authRepositoryProvider);
    await repo.saveToken(token);
    final current = state.value ?? AuthConfig.empty;
    state = AsyncData(current.copyWith(token: token));
  }

  Future<void> clearToken() async {
    final repo = ref.read(authRepositoryProvider);
    await repo.clearToken();
    final current = state.value ?? AuthConfig.empty;
    state = AsyncData(current.copyWith(clearToken: true));
  }

  Future<void> clearAll() async {
    final repo = ref.read(authRepositoryProvider);
    await repo.clearAll();
    state = const AsyncData(AuthConfig.empty);
  }
}
