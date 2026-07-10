import 'package:dio/dio.dart';

import '../models/api_error.dart';
import '../models/auth_session.dart';
import '../models/user.dart';
import '../utils/url_utils.dart';
import 'api_client.dart';

class AuthApi {
  AuthApi(this._client);

  final ApiClient _client;

  Future<User> getMe() {
    return _client.getData('/me', parser: (json) => User.fromJson(json as Map<String, dynamic>));
  }

  Future<AuthSession> login({
    required String username,
    required String password,
  }) {
    return _client.postData(
      '/auth/login',
      data: {'username': username, 'password': password},
      parser: (json) => AuthSession.fromJson(json as Map<String, dynamic>),
    );
  }

  Future<AuthSession> register({
    required String username,
    required String password,
    String? displayName,
    String? birthday,
    required String timezone,
  }) {
    return _client.postData(
      '/auth/register',
      data: {
        'username': username,
        'password': password,
        if (displayName != null && displayName.isNotEmpty)
          'display_name': displayName,
        if (birthday != null) 'birthday': birthday,
        'timezone': timezone,
      },
      parser: (json) => AuthSession.fromJson(json as Map<String, dynamic>),
    );
  }
}

Future<void> testServerConnection(String serverUrl) async {
  final dio = Dio(
    BaseOptions(
      baseUrl: buildApiBaseUrl(serverUrl),
      connectTimeout: const Duration(seconds: 15),
      receiveTimeout: const Duration(seconds: 15),
    ),
  );

  try {
    final response = await dio.get<dynamic>('/health');
    final body = response.data;
    if (body is! Map<String, dynamic>) {
      throw const ApiException(
        statusCode: null,
        error: ApiError(code: 'BAD_RESPONSE', message: 'Invalid response'),
      );
    }

    final error = body['error'];
    if (error is Map<String, dynamic>) {
      throw ApiException(
        statusCode: response.statusCode,
        error: ApiError.fromJson(error),
      );
    }

    if (body['data'] != 'ok') {
      throw const ApiException(
        statusCode: null,
        error: ApiError(code: 'BAD_RESPONSE', message: 'Health check failed'),
      );
    }
  } on DioException catch (error) {
    throw ApiException(
      statusCode: error.response?.statusCode,
      error: ApiError(
        code: 'NETWORK_ERROR',
        message: error.message ?? '无法连接服务器',
      ),
    );
  }
}

String normalizeServerUrl(String input) {
  final trimmed = input.trim();
  if (trimmed.isEmpty) {
    return trimmed;
  }
  return trimmed.endsWith('/') ? trimmed.substring(0, trimmed.length - 1) : trimmed;
}

bool isValidServerUrl(String input) {
  final uri = Uri.tryParse(normalizeServerUrl(input));
  if (uri == null || !uri.hasScheme || uri.host.isEmpty) {
    return false;
  }
  return uri.scheme == 'http' || uri.scheme == 'https';
}
