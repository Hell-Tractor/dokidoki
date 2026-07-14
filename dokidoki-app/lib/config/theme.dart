import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Dokidoki 亮色主题：高饱和樱花粉 / 珊瑚，AppBar 与页面底区分开。
class AppTheme {
  AppTheme._();

  static const Color _primary = Color(0xFFFF2E96);
  static const Color _primaryDark = Color(0xFFE0187C);
  static const Color _secondary = Color(0xFFFF6A45);
  static const Color _scaffoldBg = Color(0xFFFFF3F7);
  static const Color _appBarBg = Color(0xFFFF3D9A);
  static const Color _surface = Color(0xFFFFFFFF);
  static const Color _onSurface = Color(0xFF3B1428);
  static const Color _muted = Color(0xFF8A4A66);
  static const Color _bubblePeer = Color(0xFFFFE4EE);
  static const Color _bubbleUser = Color(0xFFFFCFE3);
  static const Color _outline = Color(0xFFFFB0CA);

  static ThemeData light() {
    final colorScheme = ColorScheme.light(
      primary: _primary,
      onPrimary: Colors.white,
      primaryContainer: _bubbleUser,
      onPrimaryContainer: _onSurface,
      secondary: _secondary,
      onSecondary: Colors.white,
      secondaryContainer: const Color(0xFFFFD8CC),
      onSecondaryContainer: _onSurface,
      tertiary: const Color(0xFFFF8F3D),
      onTertiary: Colors.white,
      surface: _surface,
      onSurface: _onSurface,
      onSurfaceVariant: _muted,
      surfaceContainerHighest: _bubblePeer,
      surfaceContainerHigh: const Color(0xFFFFEAF1),
      surfaceContainer: const Color(0xFFFFF0F5),
      surfaceContainerLow: _scaffoldBg,
      outline: _outline,
      outlineVariant: const Color(0xFFFFD0E0),
      error: const Color(0xFFE53935),
      onError: Colors.white,
    );

    return ThemeData(
      colorScheme: colorScheme,
      useMaterial3: true,
      scaffoldBackgroundColor: _scaffoldBg,
      dividerColor: _outline.withValues(alpha: 0.45),
      appBarTheme: AppBarTheme(
        centerTitle: true,
        backgroundColor: _appBarBg,
        foregroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 2,
        shadowColor: _primaryDark.withValues(alpha: 0.35),
        surfaceTintColor: Colors.transparent,
        iconTheme: const IconThemeData(color: Colors.white),
        actionsIconTheme: const IconThemeData(color: Colors.white),
        titleTextStyle: const TextStyle(
          color: Colors.white,
          fontSize: 18,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.2,
        ),
        systemOverlayStyle: SystemUiOverlayStyle.light,
      ),
      floatingActionButtonTheme: const FloatingActionButtonThemeData(
        backgroundColor: _primary,
        foregroundColor: Colors.white,
        elevation: 3,
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: _primary,
          foregroundColor: Colors.white,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(14),
          ),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(foregroundColor: _primaryDark),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: _surface,
        hintStyle: TextStyle(color: _muted.withValues(alpha: 0.85)),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(24),
          borderSide: const BorderSide(color: _outline),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(24),
          borderSide: BorderSide(color: _outline.withValues(alpha: 0.8)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(24),
          borderSide: const BorderSide(color: _primary, width: 1.6),
        ),
      ),
      cardTheme: CardThemeData(
        color: _surface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFFFFD0E0)),
        ),
      ),
      bottomSheetTheme: const BottomSheetThemeData(
        backgroundColor: _surface,
        surfaceTintColor: Colors.transparent,
      ),
      snackBarTheme: SnackBarThemeData(
        backgroundColor: _onSurface,
        contentTextStyle: const TextStyle(color: Colors.white),
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
      listTileTheme: const ListTileThemeData(
        iconColor: _primaryDark,
      ),
    );
  }
}
