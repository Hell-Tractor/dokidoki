import 'package:web_socket_channel/web_socket_channel.dart';

import 'ws_connector_stub.dart'
    if (dart.library.io) 'ws_connector_io.dart'
    if (dart.library.html) 'ws_connector_web.dart';

WebSocketChannel connectWs(Uri uri, Map<String, String> headers) =>
    connectWsImpl(uri, headers);
