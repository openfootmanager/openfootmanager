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
      onClick={toggleTheme}
      className={`p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-200 dark:hover:bg-navy-600 hover:cursor-pointer transition-all duration-200 ${className}`}
      title={toggleThemeLabel}
      aria-label={toggleThemeLabel}
    >
      {isDark ? <Sun className="w-5 h-5" /> : <Moon className="w-5 h-5" />}
    </button>
  );
}
