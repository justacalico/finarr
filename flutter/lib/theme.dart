import 'package:flutter/material.dart';

/// Finarr design tokens. Restrained, dark-first, Apple-flavored:
/// near-black surfaces, one accent, generous spacing, soft corners,
/// no gradients, no shadows where a hairline border works better.
class F {
  F._();

  static const accent = Color(0xFF5E5CE6);
  static const accentSoft = Color(0x225E5CE6);
  static const ok = Color(0xFF30D158);
  static const warn = Color(0xFFFF9F0A);
  static const bad = Color(0xFFFF453A);
  static const info = Color(0xFF64D2FF);

  static const bgDark = Color(0xFF0C0C0E);
  static const surfaceDark = Color(0xFF161618);
  static const cardDark = Color(0xFF1C1C1F);
  static const borderDark = Color(0xFF2C2C2E);

  static const bgLight = Color(0xFFF5F5F7);
  static const surfaceLight = Color(0xFFFFFFFF);
  static const cardLight = Color(0xFFFFFFFF);
  static const borderLight = Color(0xFFE1E1E4);

  static const radius = 12.0;
  static const radiusS = 8.0;
  static const gap = 16.0;

  static ThemeData dark() => _base(Brightness.dark);
  static ThemeData light() => _base(Brightness.light);

  static ThemeData _base(Brightness b) {
    final dark = b == Brightness.dark;
    final scheme = ColorScheme.fromSeed(
      seedColor: accent,
      brightness: b,
      surface: dark ? surfaceDark : surfaceLight,
      primary: accent,
    );
    return ThemeData(
      useMaterial3: true,
      brightness: b,
      colorScheme: scheme,
      scaffoldBackgroundColor: dark ? bgDark : bgLight,
      fontFamily: 'system-ui',
      dividerTheme: DividerThemeData(
        color: dark ? borderDark : borderLight,
        thickness: 1,
        space: 1,
      ),
      cardTheme: CardThemeData(
        color: dark ? cardDark : cardLight,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(radius),
          side: BorderSide(color: dark ? borderDark : borderLight),
        ),
      ),
      dialogTheme: DialogThemeData(
        backgroundColor: dark ? surfaceDark : surfaceLight,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: dark ? cardDark : const Color(0xFFF0F0F2),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(radiusS),
          borderSide: BorderSide(color: dark ? borderDark : borderLight),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(radiusS),
          borderSide: BorderSide(color: dark ? borderDark : borderLight),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(radiusS),
          borderSide: const BorderSide(color: accent, width: 1.5),
        ),
        contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
        isDense: true,
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: accent,
          foregroundColor: Colors.white,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(radiusS),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(radiusS),
          ),
          side: BorderSide(color: dark ? borderDark : borderLight),
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
        ),
      ),
      navigationRailTheme: NavigationRailThemeData(
        backgroundColor: dark ? surfaceDark : surfaceLight,
        selectedIconTheme: const IconThemeData(color: accent),
        selectedLabelTextStyle: const TextStyle(
          color: accent,
          fontWeight: FontWeight.w600,
        ),
        indicatorColor: accentSoft,
      ),
      navigationBarTheme: NavigationBarThemeData(
        backgroundColor: dark ? surfaceDark : surfaceLight,
        indicatorColor: accentSoft,
        iconTheme: WidgetStateProperty.resolveWith(
          (s) => s.contains(WidgetState.selected)
              ? const IconThemeData(color: accent)
              : null,
        ),
      ),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(radiusS),
        ),
      ),
      listTileTheme: const ListTileThemeData(
        dense: true,
        contentPadding: EdgeInsets.symmetric(horizontal: 16),
      ),
      tabBarTheme: TabBarThemeData(
        indicatorColor: accent,
        labelColor: accent,
        unselectedLabelColor: dark ? Colors.white54 : Colors.black54,
        dividerColor: dark ? borderDark : borderLight,
      ),
      progressIndicatorTheme: const ProgressIndicatorThemeData(
        color: accent,
        linearTrackColor: Color(0xFF2C2C2E),
        borderRadius: BorderRadius.all(Radius.circular(4)),
      ),
      switchTheme: SwitchThemeData(
        thumbColor: WidgetStateProperty.resolveWith(
          (s) => s.contains(WidgetState.selected) ? accent : null,
        ),
        trackColor: WidgetStateProperty.resolveWith(
          (s) => s.contains(WidgetState.selected) ? accentSoft : null,
        ),
      ),
      tooltipTheme: TooltipThemeData(
        decoration: BoxDecoration(
          color: dark ? const Color(0xFF2C2C2E) : Colors.white,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: dark ? borderDark : borderLight),
        ),
        textStyle: TextStyle(
          color: dark ? Colors.white : Colors.black87,
          fontSize: 12,
        ),
      ),
      popupMenuTheme: PopupMenuThemeData(
        color: dark ? cardDark : cardLight,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(color: dark ? borderDark : borderLight),
        ),
      ),
      checkboxTheme: CheckboxThemeData(
        fillColor: WidgetStateProperty.resolveWith(
          (s) => s.contains(WidgetState.selected) ? accent : null,
        ),
        side: BorderSide(color: dark ? Colors.white38 : Colors.black38),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
      ),
    );
  }
}
