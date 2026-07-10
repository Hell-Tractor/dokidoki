import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../config/api_config.dart';
import 'auth_config.dart';

class AuthRepository {
  AuthRepository({
    required FlutterSecureStorage secureStorage,
    required SharedPreferences preferences,
  })  : _secureStorage = secureStorage,
        _preferences = preferences;

  final FlutterSecureStorage _secureStorage;
  final SharedPreferences _preferences;

  Future<AuthConfig> load() async {
    final serverUrl = _preferences.getString(serverUrlStorageKey);
    final token = await _secureStorage.read(key: tokenStorageKey);
    return AuthConfig(serverUrl: serverUrl, token: token);
  }

  Future<void> saveServerUrl(String serverUrl) async {
    await _preferences.setString(serverUrlStorageKey, serverUrl);
  }

  Future<void> saveToken(String token) async {
    await _secureStorage.write(key: tokenStorageKey, value: token);
  }

  Future<void> clearToken() async {
    await _secureStorage.delete(key: tokenStorageKey);
  }

  Future<void> clearAll() async {
    await _preferences.remove(serverUrlStorageKey);
    await clearToken();
  }
}
