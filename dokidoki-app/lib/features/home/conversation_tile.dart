import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/auth/providers.dart';
import '../../core/models/conversation.dart';
import '../../core/utils/format_utils.dart';
import '../../core/utils/url_utils.dart';
import '../../shared/widgets/character_avatar.dart';

class ConversationTile extends ConsumerWidget {
  const ConversationTile({
    super.key,
    required this.item,
    required this.onTap,
  });

  final ConversationListItem item;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final serverUrl = ref.watch(authConfigProvider).value?.serverUrl;
    final avatarUrl = serverUrl == null
        ? null
        : resolveServerResource(
            serverUrl,
            '/api/v1/characters/${item.characterId}/avatar',
          );

    final lastMessage = item.lastMessage;
    final subtitle = lastMessage == null
        ? '暂无消息'
        : truncatePreview(lastMessage.content);
    final trailing = lastMessage == null
        ? null
        : formatMessageTime(lastMessage.createdAt);

    return ListTile(
      leading: CharacterAvatar(
        name: item.characterName,
        imageUrl: avatarUrl,
      ),
      title: Text(item.characterName),
      subtitle: Text(
        subtitle,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: trailing == null
          ? null
          : Text(
              trailing,
              style: Theme.of(context).textTheme.bodySmall,
            ),
      onTap: onTap,
    );
  }
}
