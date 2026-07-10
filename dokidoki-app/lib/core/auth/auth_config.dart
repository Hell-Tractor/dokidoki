import '../utils/url_utils.dart';

class AuthConfig {
  const AuthConfig({this.serverUrl, this.token});

  final String? serverUrl;
  final String? token;

  static const empty = AuthConfig();

  bool get hasServerUrl => serverUrl != null && serverUrl!.isNotEmpty;

  bool get hasToken => token != null && token!.isNotEmpty;

  bool get isAuthenticated => hasServerUrl && hasToken;

  String? get apiBaseUrl =>
      hasServerUrl ? buildApiBaseUrl(serverUrl!) : null;

  String? get wsUrl => hasServerUrl ? buildWsUrl(serverUrl!) : null;

  AuthConfig copyWith({String? serverUrl, String? token, bool clearToken = false}) {
    return AuthConfig(
      serverUrl: serverUrl ?? this.serverUrl,
      token: clearToken ? null : (token ?? this.token),
    );
  }
}
