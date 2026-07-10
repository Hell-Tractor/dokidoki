class ApiError {
  const ApiError({required this.code, required this.message});

  final String code;
  final String message;

  factory ApiError.fromJson(Map<String, dynamic> json) {
    return ApiError(
      code: json['code'] as String,
      message: json['message'] as String,
    );
  }
}

class ApiException implements Exception {
  const ApiException({required this.statusCode, required this.error});

  final int? statusCode;
  final ApiError error;

  @override
  String toString() => 'ApiException(${error.code}): ${error.message}';
}
