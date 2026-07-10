class User {
  const User({
    required this.id,
    required this.username,
    required this.displayName,
    this.birthday,
    required this.timezone,
    required this.maxProactivePerDay,
  });

  final String id;
  final String username;
  final String displayName;
  final String? birthday;
  final String timezone;
  final int maxProactivePerDay;

  factory User.fromJson(Map<String, dynamic> json) {
    return User(
      id: json['id'] as String,
      username: json['username'] as String,
      displayName: json['display_name'] as String,
      birthday: json['birthday'] as String?,
      timezone: json['timezone'] as String,
      maxProactivePerDay: json['max_proactive_per_day'] as int,
    );
  }
}
