import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/auth/providers.dart';
import '../../core/models/character.dart';
import '../../core/utils/url_utils.dart';
import '../../shared/widgets/character_avatar.dart';
import 'providers.dart';

class CharacterPickerSheet extends ConsumerWidget {
  const CharacterPickerSheet({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final charactersAsync = ref.watch(charactersProvider);

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              '选择角色',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            charactersAsync.when(
              loading: () => const Center(
                child: Padding(
                  padding: EdgeInsets.all(24),
                  child: CircularProgressIndicator(),
                ),
              ),
              error: (error, _) => Padding(
                padding: const EdgeInsets.all(16),
                child: Text('加载失败：$error'),
              ),
              data: (characters) {
                if (characters.isEmpty) {
                  return const Padding(
                    padding: EdgeInsets.all(16),
                    child: Text('暂无可用角色'),
                  );
                }

                return Flexible(
                  child: ListView.separated(
                    shrinkWrap: true,
                    itemCount: characters.length,
                    separatorBuilder: (_, _) => const Divider(height: 1),
                    itemBuilder: (context, index) {
                      return _CharacterTile(character: characters[index]);
                    },
                  ),
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _CharacterTile extends ConsumerWidget {
  const _CharacterTile({required this.character});

  final Character character;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final serverUrl = ref.watch(authConfigProvider).value?.serverUrl;
    final avatarUrl = serverUrl == null
        ? null
        : resolveServerResource(serverUrl, character.avatarUrl);

    return ListTile(
      leading: CharacterAvatar(
        name: character.name,
        imageUrl: avatarUrl,
      ),
      title: Text(character.name),
      onTap: () => Navigator.of(context).pop(character),
    );
  }
}
