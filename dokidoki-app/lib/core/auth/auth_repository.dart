import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../config/api_config.dart';
import 'auth_config.dart';

class AuthRepository {
  AuthRepository({
    required this.secureStorage,
    required this.preferences,
  });

  final FlutterSecureStorage secureStorage;
  final SharedPreferences preferences;

  Future<AuthConfig> load() async {
    final serverUrl = preferences.getString(serverUrlStorageKey);
    final token = await secureStorage.read(key: tokenStorageKey);
    return AuthConfig(serverUrl: serverUrl, token: token);
  }

  Future<void> saveServerUrl(String serverUrl) async {
    await preferences.setString(serverUrlStorageKey, serverUrl);
  }

  Future<void> saveToken(String token) async {
    await secureStorage.write(key: tokenStorageKey, value: token);
  }

  Future<void> clearToken() async {
    await secureStorage.delete(key: tokenStorageKey);
  }

  Future<void> clearAll() async {
    await preferences.remove(serverUrlStorageKey);
    await clearToken();
  }
}
