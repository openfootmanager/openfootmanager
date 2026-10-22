import { useTranslation } from "react-i18next";
import { useCallback, useEffect, useId, useRef, type JSX, type ReactNode } from "react";
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
import type { AppSettings } from "../../store/settingsStore";
import { UnreadBadge } from "../ui";
import type { SidebarLayout } from "./sidebarLayout";
import { useSidebar } from "./useSidebar";

interface DashboardSidebarProps {
  activeTab: string;
  /** The open width, from Settings. */
  width: AppSettings["sidebar_width"];
  /** How it opens, from Settings: as today, on hover, on a click, or never (icons only). */
  behaviour: AppSettings["sidebar_behaviour"];
  /** The player folded it (Normal), remembered in Settings. */
  collapsed: boolean;
  /** `fromDrawer` when the page was chosen from the drawer, so the page heading takes the focus. */
  onNavClick: (tab: string, options?: { fromDrawer: boolean }) => void;
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

/** The width classes for each width the layout gives (the `--spacing-sidebar-*` tokens). */
const WIDTH_CLASS: Record<SidebarLayout["slot"], string> = {
  rail: "w-sidebar-rail",
  compact: "w-sidebar-compact",
  standard: "w-sidebar-standard",
  wide: "w-sidebar-wide",
};

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
      aria-current={active ? "page" : undefined}
      title={collapsed ? label : undefined}
      aria-label={
        badge !== undefined && badge !== 0 && badge !== "" ? `${label} (${badge})` : label
      }
      className={buttonClassName}
    >
      <div className={`flex items-center ${collapsed ? "justify-center" : "gap-3"}`}>
        <div className="[&>svg]:w-5 [&>svg]:h-5">{icon}</div>
        {collapsed ? null : (
          <span className="text-left font-heading font-semibold text-body uppercase leading-tight tracking-label">
            {label}
          </span>
        )}
      </div>
      {badge !== undefined && badge !== 0 && badge !== "" && (
        // On the rail the count sits on the icon's top-right corner, ringed in the chrome colour.
        <UnreadBadge
          count={badge}
          className={collapsed ? "absolute left-1/2 top-1 ml-1 ring-2 ring-chrome" : ""}
        />
      )}
    </button>
  );
}

export default function DashboardSidebar({
  activeTab,
  width,
  behaviour,
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
  const toggleRef = useRef<HTMLButtonElement>(null);
  const navRef = useRef<HTMLElement>(null);
  const focusToggle = useCallback(() => toggleRef.current?.focus(), []);
  const { layout, asideRef, asideHandlers, toggle, close } = useSidebar(
    { width, behaviour, collapsed },
    focusToggle,
  );
  const rail = layout.drawn === "rail";
  // A drawer opened from the button takes the focus to the page it is on.
  const openedByButton = useRef(false);
  useEffect(() => {
    if (!layout.overlay || !openedByButton.current) return;
    openedByButton.current = false;
    const nav = navRef.current;
    (
      nav?.querySelector<HTMLElement>('[aria-current="page"]') ?? nav?.querySelector("button")
    )?.focus();
  }, [layout.overlay]);

  function handleNav(tab: string) {
    if (layout.overlay && layout.opensDrawer) {
      close();
      onNavClick(tab, { fromDrawer: true });
      return;
    }
    onNavClick(tab);
  }

  function handleToggle() {
    if (!layout.opensDrawer) {
      onToggleCollapse();
      return;
    }
    openedByButton.current = !layout.overlay;
    toggle();
  }

  const toggleSidebarLabel =
    layout.toggle === "expand" ? t("dashboard.expandSidebar") : t("dashboard.collapseSidebar");

  return (
    <>
      {/* The slot is the width the sidebar takes from the page; open over the page, the sidebar is
          drawn wider than its slot. */}
      <div
        data-sidebar-slot
        className={`sticky top-0 z-40 h-screen shrink-0 transition-[width] duration-200 motion-reduce:transition-none ${WIDTH_CLASS[layout.slot]}`}
      >
        {/* `dark`: the sidebar is chrome, dark in both themes, so its role tokens take their dark
        values (the light focus colour would miss 3:1 on navy). */}
        <aside
          ref={asideRef}
          {...asideHandlers}
          className={`dark absolute inset-y-0 left-0 flex flex-col border-r border-line bg-chrome text-fg transition-[width] duration-200 motion-reduce:transition-none ${WIDTH_CLASS[layout.drawn]} ${
            layout.overlay ? "shadow-2xl" : ""
          }`}
        >
          {/* Brand */}
          <div className={`border-b border-line ${rail ? "px-2.5 py-4" : "p-5"}`}>
            <div
              className={`flex ${rail ? "flex-col items-center gap-3" : "items-center justify-between gap-3"}`}
            >
              <div className={`flex items-center ${rail ? "justify-center" : "gap-2"}`}>
                <div className="w-8 h-8 flex items-center justify-center">
                  <img src="../../openfootball.svg" alt={appName} className="w-8 h-8" />
                </div>
                {rail ? null : (
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
              {layout.toggle ? (
                <button
                  ref={toggleRef}
                  type="button"
                  onClick={handleToggle}
                  aria-expanded={layout.opensDrawer ? layout.overlay : undefined}
                  title={toggleSidebarLabel}
                  aria-label={toggleSidebarLabel}
                  className={`rounded-lg p-2 text-fg-muted transition-colors hover:bg-hover hover:text-fg ${FOCUS_RING}`}
                >
                  {layout.toggle === "expand" ? (
                    <PanelLeftOpen className="h-5 w-5" />
                  ) : (
                    <PanelLeftClose className="h-5 w-5" />
                  )}
                </button>
              ) : null}
            </div>
            <button
              type="button"
              onClick={() => handleNav("Manager")}
              title={rail ? t("dashboard.manager") : undefined}
              aria-label={t("dashboard.manager")}
              // The label names the action; the description carries the lines it would otherwise hide.
              aria-describedby={
                rail
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
                rail
                  ? "flex justify-center px-0 py-2 text-fg-muted"
                  : "-mx-1 border-t border-line px-1 py-1 pt-3 text-left"
              }`}
            >
              {rail ? (
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
            ref={navRef}
            className={`flex flex-1 flex-col gap-1 overflow-y-auto py-4 ${rail ? "px-1 [scrollbar-width:none]" : "px-3"}`}
          >
            <NavItem
              icon={<LayoutDashboard />}
              label={t("dashboard.home")}
              badge={undefined}
              active={activeTab === "Home"}
              collapsed={rail}
              onClick={() => handleNav("Home")}
            />
            <NavItem
              icon={<Mail />}
              label={t("dashboard.inbox")}
              badge={unreadMessagesCount > 0 ? unreadMessagesCount : undefined}
              active={activeTab === "Inbox"}
              collapsed={rail}
              onClick={() => handleNav("Inbox")}
            />
            <NavItem
              icon={<Newspaper />}
              label={t("dashboard.news")}
              active={activeTab === "News"}
              collapsed={rail}
              onClick={() => handleNav("News")}
            />
            <NavItem
              icon={<CalendarIcon />}
              label={t("dashboard.schedule")}
              badge={todayHasMatch ? "!" : undefined}
              active={activeTab === "Schedule"}
              collapsed={rail}
              onClick={() => handleNav("Schedule")}
            />

            {!isUnemployed && (
              <>
                <hr className="my-2 border-line" />
                {rail ? null : (
                  <p className="px-3 py-1 text-caption text-fg-muted">
                    {t("dashboard.sectionClub")}
                  </p>
                )}
                {clubItems.map((item) => (
                  <NavItem
                    key={item.tab}
                    icon={item.icon}
                    label={item.label}
                    active={activeTab === item.tab}
                    collapsed={rail}
                    onClick={() => handleNav(item.tab)}
                  />
                ))}
              </>
            )}

            <hr className="my-2 border-line" />
            {rail ? null : (
              <p className="px-3 py-1 text-caption text-fg-muted">{t("dashboard.sectionWorld")}</p>
            )}
            {worldItems.map((item) => (
              <NavItem
                key={item.tab}
                icon={item.icon}
                label={item.label}
                active={activeTab === item.tab}
                collapsed={rail}
                onClick={() => handleNav(item.tab)}
              />
            ))}
          </nav>

          {/* Settings & Exit */}
          <div className={`flex flex-col gap-1 border-t border-line ${rail ? "p-1" : "p-3"}`}>
            <button
              type="button"
              onClick={onNavigateSettings}
              title={rail ? t("dashboard.settings") : undefined}
              aria-label={t("dashboard.settings")}
              className={`w-full rounded-lg p-3 text-fg-muted transition-colors hover:bg-hover hover:text-fg ${FOCUS_RING} ${
                rail ? "flex items-center justify-center" : "flex items-center gap-3"
              }`}
            >
              <Settings className="w-5 h-5" />
              {rail ? null : (
                <span className="font-heading text-body uppercase tracking-label">
                  {t("dashboard.settings")}
                </span>
              )}
            </button>
            <button
              type="button"
              onClick={onExitClick}
              title={rail ? t("dashboard.exitToMenu") : undefined}
              aria-label={t("dashboard.exitToMenu")}
              className={`w-full rounded-lg p-3 text-fg-muted transition-colors hover:bg-danger-tint hover:text-danger ${FOCUS_RING} ${
                rail ? "flex items-center justify-center" : "flex items-center gap-3"
              }`}
            >
              <LogOut className="w-5 h-5" />
              {rail ? null : (
                <span className="font-heading text-body uppercase tracking-label">
                  {t("dashboard.exitToMenu")}
                </span>
              )}
            </button>
          </div>
        </aside>
      </div>
    </>
  );
}
