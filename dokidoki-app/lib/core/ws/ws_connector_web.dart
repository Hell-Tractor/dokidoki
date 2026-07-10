import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel connectWsImpl(Uri uri, Map<String, String> headers) {
  return WebSocketChannel.connect(uri);
}
