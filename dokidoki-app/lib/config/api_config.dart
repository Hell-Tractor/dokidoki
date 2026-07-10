/// REST API path prefix appended to the user-configured server URL.
const String apiPathPrefix = '/api/v1';

/// WebSocket path relative to the server host.
const String wsPath = '/api/v1/ws';

/// Secure storage key for the bearer token.
const String tokenStorageKey = 'auth_token';

/// SharedPreferences key for the server base URL (scheme + host + port).
const String serverUrlStorageKey = 'server_url';
