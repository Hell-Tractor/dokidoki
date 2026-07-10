import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

import 'ws_connector.dart';

enum WsConnectionState { disconnected, connecting, connected }

class WsEvent {
  const WsEvent({required this.type, required this.payload});

  final String type;
  final Map<String, dynamic> payload;

  factory WsEvent.fromJson(Map<String, dynamic> json) {
    return WsEvent(
      type: json['type'] as String,
      payload: (json['payload'] as Map<String, dynamic>?) ?? const {},
    );
  }
}

class WsClient {
  WsClient({
    required Future<String?> Function() this._getToken,
    required String? Function() this._getWsUrl,
  });

  final Future<String?> Function() _getToken;
  final String? Function() _getWsUrl;

  WebSocketChannel? _channel;
  StreamSubscription<dynamic>? _subscription;
  Timer? _pingTimer;
  Timer? _reconnectTimer;
  int _reconnectAttempt = 0;
  bool _disposed = false;
  bool _manualDisconnect = false;

  final _eventsController = StreamController<WsEvent>.broadcast();
  final _stateController = StreamController<WsConnectionState>.broadcast();

  Stream<WsEvent> get events => _eventsController.stream;

  Stream<WsConnectionState> get connectionState => _stateController.stream;

  WsConnectionState _state = WsConnectionState.disconnected;

  WsConnectionState get state => _state;

  Future<void> connect() async {
    if (_disposed) {
      return;
    }

    _manualDisconnect = false;
    _reconnectTimer?.cancel();

    final wsUrl = _getWsUrl();
    final token = await _getToken();
    if (wsUrl == null || token == null || token.isEmpty) {
      await disconnect();
      return;
    }

    await _closeChannel();
    _setState(WsConnectionState.connecting);

    try {
      final channel = connectWs(
        Uri.parse(wsUrl),
        {'Authorization': 'Bearer $token'},
      );
      _channel = channel;
      _reconnectAttempt = 0;
      _setState(WsConnectionState.connected);
      _startPing();

      _subscription = channel.stream.listen(
        _onMessage,
        onError: (_) => _scheduleReconnect(),
        onDone: _scheduleReconnect,
        cancelOnError: true,
      );
    } catch (_) {
      _setState(WsConnectionState.disconnected);
      _scheduleReconnect();
    }
  }

  Future<void> disconnect() async {
    _manualDisconnect = true;
    _reconnectTimer?.cancel();
    _pingTimer?.cancel();
    await _closeChannel();
    _setState(WsConnectionState.disconnected);
  }

  void subscribe(String conversationId) {
    _send({
      'type': 'subscribe',
      'payload': {'conversation_id': conversationId},
    });
  }

  void sendPing() {
    _send({'type': 'ping'});
  }

  void dispose() {
    _disposed = true;
    _manualDisconnect = true;
    _reconnectTimer?.cancel();
    _pingTimer?.cancel();
    _closeChannel();
    _eventsController.close();
    _stateController.close();
  }

  void _onMessage(dynamic message) {
    if (message is! String) {
      return;
    }

    try {
      final json = jsonDecode(message) as Map<String, dynamic>;
      final event = WsEvent.fromJson(json);
      if (event.type == 'pong') {
        return;
      }
      _eventsController.add(event);
    } catch (_) {
      // Ignore malformed frames.
    }
  }

  void _send(Map<String, dynamic> message) {
    final channel = _channel;
    if (channel == null || _state != WsConnectionState.connected) {
      return;
    }
    channel.sink.add(jsonEncode(message));
  }

  void _startPing() {
    _pingTimer?.cancel();
    _pingTimer = Timer.periodic(const Duration(seconds: 30), (_) => sendPing());
  }

  void _scheduleReconnect() {
    if (_disposed || _manualDisconnect) {
      return;
    }

    _pingTimer?.cancel();
    _closeChannel();
    _setState(WsConnectionState.disconnected);

    _reconnectTimer?.cancel();
    final delay = Duration(
      seconds: (1 << _reconnectAttempt).clamp(1, 30),
    );
    _reconnectAttempt = (_reconnectAttempt + 1).clamp(0, 5);

    _reconnectTimer = Timer(delay, connect);
  }

  Future<void> _closeChannel() async {
    await _subscription?.cancel();
    _subscription = null;
    await _channel?.sink.close();
    _channel = null;
  }

  void _setState(WsConnectionState next) {
    _state = next;
    if (!_stateController.isClosed) {
      _stateController.add(next);
    }
  }
}
