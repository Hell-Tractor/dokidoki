import 'package:dio/dio.dart';

import '../models/api_error.dart';

class ApiClient {
  ApiClient({
    required String baseUrl,
    required this._getToken,
    required this._onUnauthorized,
  }) : _dio = Dio(
          BaseOptions(
            baseUrl: baseUrl,
            connectTimeout: const Duration(seconds: 15),
            receiveTimeout: const Duration(seconds: 30),
            headers: {'Accept': 'application/json'},
          ),
        ) {
    _dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: (options, handler) async {
          final token = await _getToken();
          if (token != null && token.isNotEmpty) {
            options.headers['Authorization'] = 'Bearer $token';
          }
          handler.next(options);
        },
        onError: (error, handler) {
          if (error.response?.statusCode == 401) {
            _onUnauthorized();
          }
          handler.next(error);
        },
      ),
    );
  }

  final Dio _dio;
  final Future<String?> Function() _getToken;
  final void Function() _onUnauthorized;

  Dio get dio => _dio;

  Future<T> getData<T>(
    String path, {
    Map<String, dynamic>? queryParameters,
    T Function(dynamic json)? parser,
  }) {
    return _request(
      () => _dio.get<dynamic>(path, queryParameters: queryParameters),
      parser: parser,
    );
  }

  Future<T> postData<T>(
    String path, {
    Object? data,
    T Function(dynamic json)? parser,
  }) {
    return _request(
      () => _dio.post<dynamic>(path, data: data),
      parser: parser,
    );
  }

  Future<T> patchData<T>(
    String path, {
    Object? data,
    T Function(dynamic json)? parser,
  }) {
    return _request(
      () => _dio.patch<dynamic>(path, data: data),
      parser: parser,
    );
  }

  Future<T> putData<T>(
    String path, {
    Object? data,
    T Function(dynamic json)? parser,
  }) {
    return _request(
      () => _dio.put<dynamic>(path, data: data),
      parser: parser,
    );
  }

  Future<void> deleteData(String path) async {
    await _request(() => _dio.delete<dynamic>(path));
  }

  Future<T> _request<T>(
    Future<Response<dynamic>> Function() send, {
    T Function(dynamic json)? parser,
  }) async {
    try {
      final response = await send();
      return _parseEnvelope(response.data, parser: parser);
    } on DioException catch (error) {
      throw _mapDioError(error);
    }
  }

  T _parseEnvelope<T>(
    dynamic body, {
    T Function(dynamic json)? parser,
  }) {
    if (body is! Map<String, dynamic>) {
      throw const ApiException(
        statusCode: null,
        error: ApiError(code: 'BAD_RESPONSE', message: 'Invalid response body'),
      );
    }

    final error = body['error'];
    if (error is Map<String, dynamic>) {
      throw ApiException(
        statusCode: null,
        error: ApiError.fromJson(error),
      );
    }

    final data = body['data'];
    if (parser != null) {
      return parser(data);
    }
    return data as T;
  }

  ApiException _mapDioError(DioException error) {
    final body = error.response?.data;
    if (body is Map<String, dynamic>) {
      final apiError = body['error'];
      if (apiError is Map<String, dynamic>) {
        return ApiException(
          statusCode: error.response?.statusCode,
          error: ApiError.fromJson(apiError),
        );
      }
    }

    return ApiException(
      statusCode: error.response?.statusCode,
      error: ApiError(
        code: 'NETWORK_ERROR',
        message: error.message ?? 'Network request failed',
      ),
    );
  }
}
