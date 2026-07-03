import { ChevronLeft, ChevronRight } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { getLocale } from "../../lib/dateFormatting";

import type { MatchdayGroup } from "../../services/scheduleService";

interface ScheduleCalendarGridProps {
  groups: MatchdayGroup[];
  userTeamId: string | null;
  today: string;
  /** Anchor date to open on (next user match date, or today). */
  focusDate: string | null;
  /** Called when user clicks a date that has fixtures. */
  onSelectDate: (date: string) => void;
}

interface CalendarDay {
  date: string;
  day: number;
  hasFixture: boolean;
  hasUserMatch: boolean;
  isToday: boolean;
  isCurrentMonth: boolean;
}

function parseYMD(dateStr: string): { year: number; month: number; day: number } {
  const [year, month, day] = dateStr.split("-").map(Number);
  return { year, month, day };
}

function ymd(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function daysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}

function firstWeekdayOfMonth(year: number, month: number): number {
  // 0=Mon ... 6=Sun (ISO week)
  const jsDay = new Date(year, month - 1, 1).getDay();
  return jsDay === 0 ? 6 : jsDay - 1;
}

export default function ScheduleCalendarGrid({
  groups,
  userTeamId,
  today,
  focusDate,
  onSelectDate,
}: ScheduleCalendarGridProps) {
  const { t, i18n } = useTranslation();

  const anchorDate = focusDate ?? today;
  const { year: anchorYear, month: anchorMonth } = parseYMD(anchorDate);

  const [viewYear, setViewYear] = useState(anchorYear);
  const [viewMonth, setViewMonth] = useState(anchorMonth);

  // Re-anchor the displayed month when the focus date changes (e.g. the next
  // user match moves into a new month after a schedule refresh) — useState
  // only seeds on mount, so without this the calendar kept showing the stale
  // month. Manual prev/next navigation is preserved until the anchor moves.
  const [lastAnchorDate, setLastAnchorDate] = useState(anchorDate);
  if (anchorDate !== lastAnchorDate) {
    setLastAnchorDate(anchorDate);
    setViewYear(anchorYear);
    setViewMonth(anchorMonth);
  }

  const fixtureDates = useMemo(() => {
    const map = new Map<string, boolean>(); // date → hasUserMatch
    for (const group of groups) {
      for (const fixture of group.fixtures) {
        const existing = map.get(fixture.date) ?? false;
        const involvesUser =
          userTeamId !== null &&
          (fixture.home_team_id === userTeamId ||
            fixture.away_team_id === userTeamId);
        map.set(fixture.date, existing || involvesUser);
      }
    }
    return map;
  }, [groups, userTeamId]);

  const days = useMemo<CalendarDay[]>(() => {
    const totalDays = daysInMonth(viewYear, viewMonth);
    const firstWeekday = firstWeekdayOfMonth(viewYear, viewMonth);
    const result: CalendarDay[] = [];

    // Padding days from previous month
    const prevMonth = viewMonth === 1 ? 12 : viewMonth - 1;
    const prevYear = viewMonth === 1 ? viewYear - 1 : viewYear;
    const prevMonthDays = daysInMonth(prevYear, prevMonth);
    for (let i = firstWeekday - 1; i >= 0; i--) {
      const d = prevMonthDays - i;
      const date = ymd(prevYear, prevMonth, d);
      result.push({
        date,
        day: d,
        hasFixture: fixtureDates.has(date),
        hasUserMatch: fixtureDates.get(date) ?? false,
        isToday: date === today,
        isCurrentMonth: false,
      });
    }

    // Current month days
    for (let d = 1; d <= totalDays; d++) {
      const date = ymd(viewYear, viewMonth, d);
      result.push({
        date,
        day: d,
        hasFixture: fixtureDates.has(date),
        hasUserMatch: fixtureDates.get(date) ?? false,
        isToday: date === today,
        isCurrentMonth: true,
      });
    }

    // Padding days from next month to complete the last week
    const remaining = (7 - (result.length % 7)) % 7;
    const nextMonth = viewMonth === 12 ? 1 : viewMonth + 1;
    const nextYear = viewMonth === 12 ? viewYear + 1 : viewYear;
    for (let d = 1; d <= remaining; d++) {
      const date = ymd(nextYear, nextMonth, d);
      result.push({
        date,
        day: d,
        hasFixture: fixtureDates.has(date),
        hasUserMatch: fixtureDates.get(date) ?? false,
        isToday: date === today,
        isCurrentMonth: false,
      });
    }

    return result;
  }, [viewYear, viewMonth, fixtureDates, today]);

  const prevMonth = () => {
    if (viewMonth === 1) {
      setViewMonth(12);
      setViewYear((y) => y - 1);
    } else {
      setViewMonth((m) => m - 1);
    }
  };

  const nextMonth = () => {
    if (viewMonth === 12) {
      setViewMonth(1);
      setViewYear((y) => y + 1);
    } else {
      setViewMonth((m) => m + 1);
    }
  };

  // Use the app language, not the OS locale, for the month name.
  const monthLabel = new Date(viewYear, viewMonth - 1, 1).toLocaleDateString(
    getLocale(i18n.language),
    { month: "long", year: "numeric" },
  );

  const weekdays = [
    t("schedule.calendar.mon", "Mo"),
    t("schedule.calendar.tue", "Tu"),
    t("schedule.calendar.wed", "We"),
    t("schedule.calendar.thu", "Th"),
    t("schedule.calendar.fri", "Fr"),
    t("schedule.calendar.sat", "Sa"),
    t("schedule.calendar.sun", "Su"),
  ];

  return (
    <div className="rounded-xl border border-gray-200 bg-white dark:border-navy-600 dark:bg-navy-800 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-100 dark:border-navy-600">
        <button
          onClick={prevMonth}
          aria-label={t("schedule.calendar.prevMonth", "Previous month")}
          className="p-1 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
        >
          <ChevronLeft className="w-4 h-4" />
        </button>
        <span className="font-heading font-bold text-sm uppercase tracking-wider text-gray-700 dark:text-gray-200">
          {monthLabel}
        </span>
        <button
          onClick={nextMonth}
          aria-label={t("schedule.calendar.nextMonth", "Next month")}
          className="p-1 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
        >
          <ChevronRight className="w-4 h-4" />
        </button>
      </div>

      {/* Weekday headers */}
      <div className="grid grid-cols-7 border-b border-gray-100 dark:border-navy-600">
        {weekdays.map((wd) => (
          <div
            key={wd}
            className="py-2 text-center font-heading text-xs font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500"
          >
            {wd}
          </div>
        ))}
      </div>

      {/* Day cells */}
      <div className="grid grid-cols-7">
        {days.map((day) => {
          const clickable = day.hasFixture;
          return (
            <button
              key={day.date}
              disabled={!clickable}
              onClick={() => clickable && onSelectDate(day.date)}
              className={[
                "relative flex flex-col items-center py-2 gap-1 text-xs font-heading font-bold transition-colors",
                day.isCurrentMonth
                  ? "text-gray-800 dark:text-gray-100"
                  : "text-gray-300 dark:text-navy-600",
                day.isToday
                  ? "bg-primary-50 dark:bg-primary-500/10"
                  : "",
                clickable
                  ? "cursor-pointer hover:bg-gray-50 dark:hover:bg-navy-700/50"
                  : "cursor-default",
              ]
                .filter(Boolean)
                .join(" ")}
              data-testid={`calendar-day-${day.date}`}
              aria-label={
                day.hasFixture
                  ? t("schedule.calendar.dayWithFixtures", "{{date}} — has fixtures", {
                      date: day.date,
                    })
                  : day.date
              }
            >
              <span
                className={
                  day.isToday
                    ? "w-6 h-6 flex items-center justify-center rounded-full bg-primary-500 text-white text-xs"
                    : ""
                }
              >
                {day.day}
              </span>
              {day.hasFixture && (
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    day.hasUserMatch
                      ? "bg-primary-500"
                      : "bg-gray-300 dark:bg-navy-500"
                  }`}
                  aria-hidden="true"
                />
              )}
            </button>
          );
        })}
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 border-t border-gray-100 dark:border-navy-600 px-4 py-2 text-xs text-gray-400 dark:text-gray-500">
        <span className="flex items-center gap-1.5">
          <span className="w-2 h-2 rounded-full bg-primary-500" />
          {t("schedule.calendar.yourMatch", "Your match")}
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-2 h-2 rounded-full bg-gray-300 dark:bg-navy-500" />
          {t("schedule.calendar.otherFixtures", "Other fixtures")}
        </span>
      </div>
    </div>
  );
}
