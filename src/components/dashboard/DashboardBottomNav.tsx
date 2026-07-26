import { useState } from "react";
import type { JSX, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  Building2,
  Calendar as CalendarIcon,
  Crosshair,
  DollarSign,
  Dumbbell,
  Eye,
  Globe,
  GraduationCap,
  LayoutDashboard,
  LogOut,
  Mail,
  Medal,
  Menu,
  Newspaper,
  Settings,
  TrendingUp,
  Trophy,
  User,
  UserCheck,
  UserCog,
  Users,
  UsersRound,
  X,
} from "lucide-react";

interface DashboardBottomNavProps {
  activeTab: string;
  isUnemployed: boolean;
  onExitClick: () => void;
  onNavigateSettings: () => void;
  onNavClick: (tab: string) => void;
  todayHasMatch?: boolean;
  unreadMessagesCount: number;
}

interface BottomNavItem {
  badge?: number | string;
  icon: ReactNode;
  label: string;
  tab: string;
}

export default function DashboardBottomNav({
  activeTab,
  isUnemployed,
  onExitClick,
  onNavigateSettings,
  onNavClick,
  todayHasMatch,
  unreadMessagesCount,
}: DashboardBottomNavProps): JSX.Element {
  const { t } = useTranslation();
  const [isMoreOpen, setIsMoreOpen] = useState(false);

  // Mirrors the item definitions/icons/i18n keys of DashboardSidebar.
  const primaryItems: BottomNavItem[] = [
    { icon: <LayoutDashboard />, label: t("dashboard.home"), tab: "Home" },
    { icon: <Users />, label: t("dashboard.squad"), tab: "Squad" },
    {
      icon: <Mail />,
      label: t("dashboard.inbox"),
      tab: "Inbox",
      badge: unreadMessagesCount > 0 ? unreadMessagesCount : undefined,
    },
    {
      icon: <CalendarIcon />,
      label: t("dashboard.schedule"),
      tab: "Schedule",
      badge: todayHasMatch ? "!" : undefined,
    },
  ];

  const clubItems: BottomNavItem[] = [
    { icon: <Crosshair />, label: t("dashboard.tactics"), tab: "Tactics" },
    { icon: <Dumbbell />, label: t("dashboard.training"), tab: "Training" },
    { icon: <UserCog />, label: t("dashboard.staff"), tab: "Staff" },
    { icon: <Eye />, label: t("dashboard.scouting"), tab: "Scouting" },
    {
      icon: <GraduationCap />,
      label: t("dashboard.youthAcademy"),
      tab: "Youth",
    },
    { icon: <DollarSign />, label: t("dashboard.finances"), tab: "Finances" },
    { icon: <TrendingUp />, label: t("dashboard.transfers"), tab: "Transfers" },
  ];

  const moreItems: BottomNavItem[] = [
    { icon: <Newspaper />, label: t("dashboard.news"), tab: "News" },
    ...(isUnemployed ? [] : clubItems),
    { icon: <Globe />, label: t("transfers.centre"), tab: "TransferCentre" },
    { icon: <Medal />, label: t("dashboard.hallOfFame"), tab: "HallOfFame" },
    { icon: <UsersRound />, label: t("dashboard.players"), tab: "Players" },
    { icon: <UserCheck />, label: t("dashboard.managers"), tab: "Managers" },
    { icon: <Building2 />, label: t("dashboard.teams"), tab: "Teams" },
    { icon: <Trophy />, label: t("dashboard.tournaments"), tab: "Tournaments" },
    { icon: <User />, label: t("dashboard.manager"), tab: "Manager" },
  ];

  const isMoreActive = moreItems.some((item) => item.tab === activeTab);

  function handleSelect(tab: string): void {
    setIsMoreOpen(false);
    onNavClick(tab);
  }

  function handleSettingsClick(): void {
    setIsMoreOpen(false);
    onNavigateSettings();
  }

  function handleExitClick(): void {
    setIsMoreOpen(false);
    onExitClick();
  }

  function renderBadge(badge: number | string | undefined): JSX.Element | null {
    if (badge === undefined || badge === 0 || badge === "") {
      return null;
    }

    return (
      <span className="absolute -right-2.5 -top-1 min-w-4 rounded-full bg-primary-500 px-1 py-0.5 text-center text-[9px] font-bold text-white">
        {badge}
      </span>
    );
  }

  return (
    <nav className="md:hidden border-t border-navy-700 bg-navy-800 pb-safe text-white">
      <div className="flex">
        {primaryItems.map((item) => (
          <button
            key={item.tab}
            type="button"
            data-testid={`bottom-nav-tab-${item.tab}`}
            aria-label={
              item.badge !== undefined && item.badge !== 0 && item.badge !== ""
                ? `${item.label} (${item.badge})`
                : item.label
            }
            onClick={() => handleSelect(item.tab)}
            className={`flex flex-1 flex-col items-center gap-1 py-2 text-[10px] font-heading font-semibold uppercase tracking-wider transition-colors ${activeTab === item.tab
                ? "text-primary-400"
                : "text-gray-400 hover:text-white"
              }`}
          >
            <span className="relative [&>svg]:h-5 [&>svg]:w-5">
              {item.icon}
              {renderBadge(item.badge)}
            </span>
            {item.label}
          </button>
        ))}
        <button
          type="button"
          data-testid="bottom-nav-more"
          aria-label={t("common.more")}
          aria-expanded={isMoreOpen}
          onClick={() => setIsMoreOpen((currentValue) => !currentValue)}
          className={`flex flex-1 flex-col items-center gap-1 py-2 text-[10px] font-heading font-semibold uppercase tracking-wider transition-colors ${isMoreActive || isMoreOpen
              ? "text-primary-400"
              : "text-gray-400 hover:text-white"
            }`}
        >
          <span className="[&>svg]:h-5 [&>svg]:w-5">
            <Menu />
          </span>
          {t("common.more")}
        </button>
      </div>

      {isMoreOpen && (
        <div
          role="dialog"
          aria-label={t("common.more")}
          className="fixed inset-0 z-50 flex flex-col justify-end"
        >
          <button
            type="button"
            aria-label={t("common.close")}
            onClick={() => setIsMoreOpen(false)}
            className="absolute inset-0 bg-black/50"
          />
          <div className="relative max-h-[70vh] overflow-y-auto rounded-t-2xl border-t border-navy-700 bg-navy-800 p-4 pb-safe">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="font-heading text-sm font-bold uppercase tracking-wider text-white">
                {t("common.more")}
              </h2>
              <button
                type="button"
                data-testid="bottom-nav-sheet-close"
                aria-label={t("common.close")}
                onClick={() => setIsMoreOpen(false)}
                className="rounded-lg p-2 text-gray-400 transition-colors hover:bg-white/5 hover:text-white"
              >
                <X className="h-5 w-5" />
              </button>
            </div>
            <div className="grid grid-cols-3 gap-2">
              {moreItems.map((item) => (
                <button
                  key={item.tab}
                  type="button"
                  data-testid={`bottom-nav-sheet-tab-${item.tab}`}
                  onClick={() => handleSelect(item.tab)}
                  className={`flex flex-col items-center gap-1.5 rounded-lg p-3 text-[10px] font-heading font-semibold uppercase tracking-wider transition-colors ${activeTab === item.tab
                      ? "bg-primary-500/20 text-primary-400"
                      : "text-gray-400 hover:bg-white/5 hover:text-white"
                    }`}
                >
                  <span className="[&>svg]:h-5 [&>svg]:w-5">{item.icon}</span>
                  {item.label}
                </button>
              ))}
            </div>
            <div className="mt-3 flex flex-col gap-1 border-t border-navy-700 pt-3">
              <button
                type="button"
                data-testid="bottom-nav-sheet-settings"
                onClick={handleSettingsClick}
                className="flex w-full items-center gap-3 rounded-lg p-3 text-gray-500 transition-colors hover:bg-white/5 hover:text-gray-300"
              >
                <Settings className="w-5 h-5" />
                <span className="font-heading text-sm uppercase tracking-wider">
                  {t("dashboard.settings")}
                </span>
              </button>
              <button
                type="button"
                data-testid="bottom-nav-sheet-exit"
                onClick={handleExitClick}
                className="flex w-full items-center gap-3 rounded-lg p-3 text-gray-500 transition-colors hover:bg-red-500/10 hover:text-red-400"
              >
                <LogOut className="w-5 h-5" />
                <span className="font-heading text-sm uppercase tracking-wider">
                  {t("dashboard.exitToMenu")}
                </span>
              </button>
            </div>
          </div>
        </div>
      )}
    </nav>
  );
}
