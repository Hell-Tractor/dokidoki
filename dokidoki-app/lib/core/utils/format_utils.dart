String formatMessageTime(String iso8601) {
  final dateTime = DateTime.parse(iso8601).toLocal();
  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);
  final messageDay = DateTime(dateTime.year, dateTime.month, dateTime.day);

  if (messageDay == today) {
    final hour = dateTime.hour.toString().padLeft(2, '0');
    final minute = dateTime.minute.toString().padLeft(2, '0');
    return '$hour:$minute';
  }
  if (messageDay == today.subtract(const Duration(days: 1))) {
    return '昨天';
  }
  return '${dateTime.month}/${dateTime.day}';
}

String truncatePreview(String text, {int maxLength = 24}) {
  if (text.length <= maxLength) {
    return text;
  }
  return '${text.substring(0, maxLength)}…';
}
