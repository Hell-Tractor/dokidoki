import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel connectWsImpl(Uri uri, Map<String, String> headers) {
  return IOWebSocketChannel.connect(uri, headers: headers);
}
