import { useTranslation } from "react-i18next";
import { useId, type JSX, type ReactNode } from "react";
import {
  Users,
  Calendar as CalendarIcon,
  Mail,
  Settings,
  LayoutDashboard,
  Medal,
  Trophy,
  TrendingUp,
  Globe,
  Crosshair,
  Dumbbell,
  DollarSign,
  Eye,
  UsersRound,
  UserCheck,
  Building2,
  UserCog,
  Newspaper,
  LogOut,
  GraduationCap,
  PanelLeftClose,
  PanelLeftOpen,
  User,
} from "lucide-react";

interface DashboardSidebarProps {
  activeTab: string;
  collapsed: boolean;
  onNavClick: (tab: string) => void;
  onToggleCollapse: () => void;
  unreadMessagesCount: number;
  todayHasMatch?: boolean;
  managerName: string | null;
  teamName: string | null;
  /** The league the club plays in, shown under the club name from every tab. */
  competitionName?: string | null;
  onNavigateSettings: () => void;
  onExitClick: () => void;
  isUnemployed: boolean;
}

interface NavItemProps {
  active?: boolean;
  badge?: number | string;
  collapsed: boolean;
  icon: ReactNode;
  label: string;
  onClick?: () => void;
}

/** A circle for one digit (as wide as it is tall); more digits widen it only by their own width. */
const UNREAD_BADGE =
  "inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-primary-700 px-1 text-caption font-bold leading-none text-on-brand tabular-nums";

const FOCUS_RING =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-chrome";

function NavItem({ active, badge, collapsed, icon, label, onClick }: NavItemProps): JSX.Element {
  const buttonClassName = `relative flex w-full items-center rounded-lg p-3 transition-colors duration-200 ${FOCUS_RING} ${
    collapsed ? "justify-center" : "justify-between"
  } ${active ? "bg-primary-700 text-on-brand" : "text-fg-muted hover:bg-hover hover:text-fg"}`;

  return (
    <button
      type="button"
      onClick={onClick}
      title={collapsed ? label : undefined}
      aria-label={
        badge !== undefined && badge !== 0 && badge !== "" ? `${label} (${badge})` : label
      }
      className={buttonClassName}
    >
      <div className={`flex items-center ${collapsed ? "justify-center" : "gap-3"}`}>
        <div className="[&>svg]:w-5 [&>svg]:h-5">{icon}</div>
        {collapsed ? null : (
          <span className="font-heading font-semibold text-body uppercase tracking-label">
            {label}
          </span>
        )}
      </div>
      {badge !== undefined && badge !== 0 && badge !== "" && (
        <span
          className={
            collapsed
              ? `absolute right-0.5 top-0.5 ${UNREAD_BADGE} ring-2 ring-chrome`
              : UNREAD_BADGE
          }
        >
          {badge}
        </span>
      )}
    </button>
  );
}

export default function DashboardSidebar({
  activeTab,
  collapsed,
  onNavClick,
  onToggleCollapse,
  unreadMessagesCount,
  todayHasMatch,
  managerName,
  teamName,
  competitionName,
  onNavigateSettings,
  onExitClick,
  isUnemployed,
}: DashboardSidebarProps): JSX.Element {
  const { t } = useTranslation();
  const managerNameId = useId();
  const teamNameId = useId();
  const competitionNameId = useId();
  const appName = t("app.name");
  const [appNamePrimary, ...appNameSecondaryParts] = appName.split(" ");
  const appNameSecondary = appNameSecondaryParts.join(" ");

  const clubItems: Array<{ icon: JSX.Element; label: string; tab: string }> = [
    { icon: <Users />, label: t("dashboard.squad"), tab: "Squad" },
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
  const worldItems: Array<{ icon: JSX.Element; label: string; tab: string }> = [
    { icon: <Globe />, label: t("transfers.centre"), tab: "TransferCentre" },
    { icon: <Medal />, label: t("dashboard.hallOfFame"), tab: "HallOfFame" },
    { icon: <UsersRound />, label: t("dashboard.players"), tab: "Players" },
    { icon: <UserCheck />, label: t("dashboard.managers"), tab: "Managers" },
    { icon: <Building2 />, label: t("dashboard.teams"), tab: "Teams" },
    {
      icon: <Trophy />,
      label: t("dashboard.tournaments"),
      tab: "Tournaments",
    },
  ];
  const toggleSidebarLabel = collapsed
    ? t("dashboard.expandSidebar")
    : t("dashboard.collapseSidebar");

  return (
    // `dark`: the sidebar is chrome, dark in both themes, so its role tokens take their dark values
    // (the light focus colour would miss 3:1 on navy).
    <aside
      className={`dark sticky top-0 flex h-screen shrink-0 flex-col border-r border-line bg-chrome text-fg transition-[width] duration-200 ${
        collapsed ? "w-20" : "w-64"
      }`}
    >
      {/* Brand */}
      <div className={`border-b border-line ${collapsed ? "px-3 py-4" : "p-5"}`}>
        <div
          className={`flex ${collapsed ? "flex-col items-center gap-3" : "items-center justify-between gap-3"}`}
        >
          <div className={`flex items-center ${collapsed ? "justify-center" : "gap-2"}`}>
            <div className="w-8 h-8 flex items-center justify-center">
              <img src="../../openfootball.svg" alt={appName} className="w-8 h-8" />
            </div>
            {collapsed ? null : (
              <div>
                <h1 className="font-heading text-body font-semibold uppercase tracking-label text-fg">
                  {appNamePrimary}
                </h1>
                {appNameSecondary ? (
                  <h1 className="font-heading text-lead font-bold uppercase tracking-label text-accent-400">
                    {appNameSecondary}
                  </h1>
                ) : null}
              </div>
            )}
          </div>
          <button
            type="button"
            onClick={onToggleCollapse}
            title={toggleSidebarLabel}
            aria-label={toggleSidebarLabel}
            className={`rounded-lg p-2 text-fg-muted transition-colors hover:bg-hover hover:text-fg ${FOCUS_RING}`}
          >
            {collapsed ? (
              <PanelLeftOpen className="h-5 w-5" />
            ) : (
              <PanelLeftClose className="h-5 w-5" />
            )}
          </button>
        </div>
        <button
          type="button"
          onClick={() => onNavClick("Manager")}
          title={collapsed ? t("dashboard.manager") : undefined}
          aria-label={t("dashboard.manager")}
          // The label names the action; the description carries the lines it would otherwise hide.
          aria-describedby={
            collapsed
              ? undefined
              : [
                  managerNameId,
                  teamName ? teamNameId : null,
                  competitionName ? competitionNameId : null,
                ]
                  .filter((id): id is string => id !== null)
                  .join(" ")
          }
          className={`mt-3 w-full rounded-lg transition-colors hover:cursor-pointer hover:bg-hover ${FOCUS_RING} ${
            collapsed
              ? "flex justify-center px-0 py-2 text-fg-muted"
              : "-mx-1 border-t border-line px-1 py-1 pt-3 text-left"
          }`}
        >
          {collapsed ? (
            <User className="h-5 w-5" />
          ) : (
            <>
              <p className="text-caption text-fg-muted">{t("dashboard.manager")}</p>
              <p id={managerNameId} className="mt-0.5 text-body font-semibold text-fg">
                {managerName}
              </p>
              {teamName && (
                <p id={teamNameId} className="mt-0.5 text-caption text-positive">
                  {teamName}
                </p>
              )}
              {competitionName && (
                <p id={competitionNameId} className="mt-0.5 text-caption text-fg-muted">
                  {competitionName}
                </p>
              )}
            </>
          )}
        </button>
      </div>

      {/* Navigation */}
      <nav
        className={`flex flex-1 flex-col gap-1 overflow-y-auto py-4 ${collapsed ? "px-2" : "px-3"}`}
      >
        <NavItem
          icon={<LayoutDashboard />}
          label={t("dashboard.home")}
          badge={undefined}
          active={activeTab === "Home"}
          collapsed={collapsed}
          onClick={() => onNavClick("Home")}
        />
        <NavItem
          icon={<Mail />}
          label={t("dashboard.inbox")}
          badge={unreadMessagesCount > 0 ? unreadMessagesCount : undefined}
          active={activeTab === "Inbox"}
          collapsed={collapsed}
          onClick={() => onNavClick("Inbox")}
        />
        <NavItem
          icon={<Newspaper />}
          label={t("dashboard.news")}
          active={activeTab === "News"}
          collapsed={collapsed}
          onClick={() => onNavClick("News")}
        />
        <NavItem
          icon={<CalendarIcon />}
          label={t("dashboard.schedule")}
          badge={todayHasMatch ? "!" : undefined}
          active={activeTab === "Schedule"}
          collapsed={collapsed}
          onClick={() => onNavClick("Schedule")}
        />

        {!isUnemployed && (
          <>
            <hr className="my-2 border-line" />
            {collapsed ? null : (
              <p className="px-3 py-1 text-caption text-fg-muted">{t("dashboard.sectionClub")}</p>
            )}
            {clubItems.map((item) => (
              <NavItem
                key={item.tab}
                icon={item.icon}
                label={item.label}
                active={activeTab === item.tab}
                collapsed={collapsed}
                onClick={() => onNavClick(item.tab)}
              />
            ))}
          </>
        )}

        <hr className="my-2 border-line" />
        {collapsed ? null : (
          <p className="px-3 py-1 text-caption text-fg-muted">{t("dashboard.sectionWorld")}</p>
        )}
        {worldItems.map((item) => (
          <NavItem
            key={item.tab}
            icon={item.icon}
            label={item.label}
            active={activeTab === item.tab}
            collapsed={collapsed}
            onClick={() => onNavClick(item.tab)}
          />
        ))}
      </nav>

      {/* Settings & Exit */}
      <div className={`flex flex-col gap-1 border-t border-line ${collapsed ? "p-2" : "p-3"}`}>
        <button
          type="button"
          onClick={onNavigateSettings}
          title={collapsed ? t("dashboard.settings") : undefined}
          aria-label={t("dashboard.settings")}
          className={`w-full rounded-lg p-3 text-fg-muted transition-colors hover:bg-hover hover:text-fg ${FOCUS_RING} ${
            collapsed ? "flex items-center justify-center" : "flex items-center gap-3"
          }`}
        >
          <Settings className="w-5 h-5" />
          {collapsed ? null : (
            <span className="font-heading text-body uppercase tracking-label">
              {t("dashboard.settings")}
            </span>
          )}
        </button>
        <button
          type="button"
          onClick={onExitClick}
          title={collapsed ? t("dashboard.exitToMenu") : undefined}
          aria-label={t("dashboard.exitToMenu")}
          className={`w-full rounded-lg p-3 text-fg-muted transition-colors hover:bg-danger-tint hover:text-danger ${FOCUS_RING} ${
            collapsed ? "flex items-center justify-center" : "flex items-center gap-3"
          }`}
        >
          <LogOut className="w-5 h-5" />
          {collapsed ? null : (
            <span className="font-heading text-body uppercase tracking-label">
              {t("dashboard.exitToMenu")}
            </span>
          )}
        </button>
      </div>
    </aside>
  );
}
