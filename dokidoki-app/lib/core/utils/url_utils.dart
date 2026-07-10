import '../../config/api_config.dart';

String buildApiBaseUrl(String serverUrl) {
  final normalized = serverUrl.endsWith('/')
      ? serverUrl.substring(0, serverUrl.length - 1)
      : serverUrl;
  return '$normalized$apiPathPrefix';
}

String buildWsUrl(String serverUrl) {
  final uri = Uri.parse(serverUrl);
  final scheme = uri.scheme == 'https' ? 'wss' : 'ws';
  return Uri(
    scheme: scheme,
    host: uri.host,
    port: uri.hasPort ? uri.port : null,
    path: wsPath,
  ).toString();
}

String resolveServerResource(String serverUrl, String path) {
  if (path.startsWith('http://') || path.startsWith('https://')) {
    return path;
  }
  final base = serverUrl.endsWith('/')
      ? serverUrl.substring(0, serverUrl.length - 1)
      : serverUrl;
  return '$base$path';
}
