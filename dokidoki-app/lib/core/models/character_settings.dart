class CharacterSettings {
  const CharacterSettings({
    this.dndStart,
    this.dndEnd,
    this.pushMuted = false,
  });

  final String? dndStart;
  final String? dndEnd;
  final bool pushMuted;

  factory CharacterSettings.fromJson(Map<String, dynamic> json) {
    return CharacterSettings(
      dndStart: json['dnd_start'] as String?,
      dndEnd: json['dnd_end'] as String?,
      pushMuted: json['push_muted'] as bool? ?? false,
    );
  }
}
