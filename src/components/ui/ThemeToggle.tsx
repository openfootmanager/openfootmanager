import { Sun, Moon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTheme } from "../../context/ThemeContext";

interface ThemeToggleProps {
  className?: string;
}

export function ThemeToggle({ className = "" }: ThemeToggleProps) {
  const { t } = useTranslation();
  const { isDark, toggleTheme } = useTheme();
  const toggleThemeLabel = isDark
    ? t("settings.switchToLightMode")
    : t("settings.switchToDarkMode");

  return (
    <button
      type="button"
      onClick={toggleTheme}
      className={`p-2 rounded-lg text-fg-muted hover:text-fg hover:bg-hover hover:cursor-pointer transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page ${className}`}
      title={toggleThemeLabel}
      aria-label={toggleThemeLabel}
    >
      {isDark ? <Sun className="w-5 h-5" /> : <Moon className="w-5 h-5" />}
    </button>
  );
}
