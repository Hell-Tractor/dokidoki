import 'package:flutter/material.dart';

import '../../../core/models/message.dart';

class MessageBubble extends StatelessWidget {
  const MessageBubble({
    super.key,
    required this.message,
    required this.showAvatar,
    required this.characterName,
    required this.userDisplayName,
    this.characterAvatarUrl,
  });

  final ChatMessage message;
  final bool showAvatar;
  final String characterName;
  final String userDisplayName;
  final String? characterAvatarUrl;

  static const double avatarSize = 36;

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    final colorScheme = Theme.of(context).colorScheme;

    final bubbleColor = isUser
        ? colorScheme.primaryContainer
        : colorScheme.surfaceContainerHighest;
    final textColor = isUser
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurface;

    final avatarLabel = isUser ? userDisplayName : characterName;
    final avatarWidget = showAvatar
        ? _Avatar(
            label: avatarLabel,
            imageUrl: isUser ? null : characterAvatarUrl,
            backgroundColor: isUser
                ? colorScheme.secondaryContainer
                : colorScheme.primaryContainer,
            foregroundColor: isUser
                ? colorScheme.onSecondaryContainer
                : colorScheme.onPrimaryContainer,
          )
        : const SizedBox(width: avatarSize, height: avatarSize);

    final bubble = Container(
      constraints: BoxConstraints(
        maxWidth: MediaQuery.sizeOf(context).width * 0.72,
      ),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: bubbleColor,
        borderRadius: BorderRadius.only(
          topLeft: const Radius.circular(16),
          topRight: const Radius.circular(16),
          bottomLeft: Radius.circular(isUser ? 16 : 4),
          bottomRight: Radius.circular(isUser ? 4 : 16),
        ),
      ),
      child: Text(
        message.displayContent,
        style: TextStyle(color: textColor),
      ),
    );

    final readIndicator = isUser && message.isRead
        ? Icon(
            Icons.done,
            size: 14,
            color: Colors.green.shade600,
          )
        : null;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        mainAxisAlignment:
            isUser ? MainAxisAlignment.end : MainAxisAlignment.start,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: isUser
            ? [
                bubble,
                if (readIndicator != null) ...[
                  const SizedBox(width: 4),
                  readIndicator,
                ],
                const SizedBox(width: 8),
                avatarWidget,
              ]
            : [avatarWidget, const SizedBox(width: 8), bubble],
      ),
    );
  }
}

class _Avatar extends StatelessWidget {
  const _Avatar({
    required this.label,
    required this.backgroundColor,
    required this.foregroundColor,
    this.imageUrl,
  });

  final String label;
  final Color backgroundColor;
  final Color foregroundColor;
  final String? imageUrl;

  @override
  Widget build(BuildContext context) {
    final initial = label.isNotEmpty ? label[0] : '?';

    return CircleAvatar(
      radius: MessageBubble.avatarSize / 2,
      backgroundColor: backgroundColor,
      backgroundImage:
          imageUrl == null ? null : NetworkImage(imageUrl!),
      onBackgroundImageError: imageUrl == null ? null : (_, _) {},
      child: Text(initial, style: TextStyle(color: foregroundColor)),
    );
  }
}

bool shouldShowAvatar(List<ChatMessage> messages, int dataIndex) {
  if (dataIndex <= 0) {
    return true;
  }
  return messages[dataIndex - 1].role != messages[dataIndex].role;
}
