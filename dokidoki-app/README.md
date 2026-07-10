# dokidoki-app

Flutter client for Dokidoki.

## Structure

```
lib/
  main.dart
  app.dart
  config/          # theme, router, API constants
  core/
    api/           # Dio client + providers
    auth/          # AuthConfig, secure storage, providers
    models/        # API-aligned models
    utils/
    ws/            # WebSocket client + reconnect
  features/
    setup/         # splash, server URL, auth
    home/          # conversation list
    chat/          # chat page
    settings/      # user & character settings
  shared/widgets/
```

## Run

```bash
flutter pub get
flutter run
```
