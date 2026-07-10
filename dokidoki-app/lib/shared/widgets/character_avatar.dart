import 'package:flutter/material.dart';

class CharacterAvatar extends StatelessWidget {
  const CharacterAvatar({
    super.key,
    required this.name,
    this.imageUrl,
    this.radius = 24,
  });

  final String name;
  final String? imageUrl;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final initial = name.isNotEmpty ? name[0] : '?';
    final colorScheme = Theme.of(context).colorScheme;

    if (imageUrl == null || imageUrl!.isEmpty) {
      return CircleAvatar(
        radius: radius,
        backgroundColor: colorScheme.primaryContainer,
        child: Text(
          initial,
          style: TextStyle(color: colorScheme.onPrimaryContainer),
        ),
      );
    }

    return CircleAvatar(
      radius: radius,
      backgroundColor: colorScheme.primaryContainer,
      backgroundImage: NetworkImage(imageUrl!),
      onBackgroundImageError: (_, _) {},
      child: Text(
        initial,
        style: TextStyle(color: colorScheme.onPrimaryContainer),
      ),
    );
  }
}
