import { useEffect, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { Briefcase, Star, MapPin, RefreshCw } from "lucide-react";
import { Card, CardHeader, CardBody } from "../ui";
import {
  getAvailableJobs,
  applyForJob,
  type JobOpportunity,
} from "../../services/jobService";
import { getTeamName } from "../../lib/helpers";
import type { GameStateData } from "../../store/gameStore";
import SwitchClubConfirmModal from "../SwitchClubConfirmModal";

interface JobOpportunitiesCardProps {
  gameState: GameStateData;
  onGameUpdate: (state: GameStateData) => void;
  hideWhenEmpty?: boolean;
}

function getReputationStars(reputation: number): number {
  return Math.min(5, Math.max(1, Math.round(reputation / 200)));
}

export default function JobOpportunitiesCard({
  gameState,
  onGameUpdate,
  hideWhenEmpty = false,
}: JobOpportunitiesCardProps): JSX.Element | null {
  const { t } = useTranslation();
  const [jobs, setJobs] = useState<JobOpportunity[]>([]);
  const [loading, setLoading] = useState(true);
  const [applyingTo, setApplyingTo] = useState<string | null>(null);
  const [pendingSwitch, setPendingSwitch] = useState<JobOpportunity | null>(
    null,
  );
  const [feedback, setFeedback] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  const currentDate = gameState.clock.current_date;
  const currentClubName = getTeamName(
    gameState.teams,
    gameState.manager?.team_id ?? null,
  );

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    getAvailableJobs()
      .then((result) => {
        if (!cancelled) setJobs(result);
      })
      .catch((err) => console.error("Failed to fetch jobs:", err))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [currentDate]);

  const submitApplication = async (job: JobOpportunity) => {
    setApplyingTo(job.team_id);
    setFeedback(null);
    try {
      const response = await applyForJob(job.team_id);
      onGameUpdate(response.game);
      if (response.result === "hired") {
        // Set success feedback first: a refresh failure below must not
        // swallow the "hired" message via the surrounding catch.
        setFeedback({ type: "success", message: t("jobs.hired") });
        // After a club switch the prior list reflects opportunities relative
        // to the old club — refetch so we don't keep showing offers that
        // would now fail the "step up" check.
        const updated = await getAvailableJobs();
        setJobs(updated);
      } else if (response.result === "rejected") {
        setFeedback({ type: "error", message: t("jobs.rejected") });
        const updated = await getAvailableJobs();
        setJobs(updated);
      } else if (response.result === "same_team") {
        setFeedback({
          type: "error",
          message: t("jobs.sameTeam"),
        });
      } else if (response.result === "not_better_club") {
        setFeedback({
          type: "error",
          message: t("jobs.notBetterClub"),
        });
      }
    } catch (err) {
      console.error("Failed to apply:", err);
    } finally {
      setApplyingTo(null);
    }
  };

  const handleApply = (job: JobOpportunity) => {
    if (applyingTo) return;
    // Only prompt for confirmation when the offer points at a different club —
    // applying to your current club is a backend no-op (returns `same_team`).
    if (
      gameState.manager?.team_id &&
      gameState.manager.team_id !== job.team_id
    ) {
      setPendingSwitch(job);
      return;
    }
    void submitApplication(job);
  };

  const handleConfirmSwitch = () => {
    if (!pendingSwitch) return;
    const job = pendingSwitch;
    setPendingSwitch(null);
    void submitApplication(job);
  };

  const handleRefresh = async () => {
    setLoading(true);
    try {
      const result = await getAvailableJobs();
      setJobs(result);
    } catch (err) {
      console.error("Failed to refresh jobs:", err);
    } finally {
      setLoading(false);
    }
  };

  if (hideWhenEmpty && !loading && jobs.length === 0 && !feedback) {
    return null;
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between w-full">
          <div className="flex items-center gap-2">
            <Briefcase className="w-4 h-4 text-primary-500" />
            {t("jobs.opportunitiesTitle")}
          </div>
          <button
            onClick={handleRefresh}
            disabled={loading}
            className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            title={t("jobs.refresh")}
          >
            <RefreshCw
              className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`}
            />
          </button>
        </div>
      </CardHeader>
      <CardBody>
        {feedback && (
          <div
            className={`mb-3 rounded-lg px-3 py-2 text-sm font-medium ${
              feedback.type === "success"
                ? "bg-green-50 text-green-700 dark:bg-green-950/30 dark:text-green-400"
                : "bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-400"
            }`}
          >
            {feedback.message}
          </div>
        )}

        {loading && jobs.length === 0 ? (
          <div className="flex items-center justify-center py-6">
            <div className="w-5 h-5 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
          </div>
        ) : jobs.length === 0 ? (
          <p className="text-sm text-gray-400 dark:text-gray-500 text-center py-4">
            {t("jobs.noJobs")}
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {jobs.map((job) => {
              const stars = getReputationStars(job.reputation);
              return (
                <div
                  key={job.team_id}
                  className="flex items-center justify-between rounded-lg border border-gray-100 bg-gray-50 px-4 py-3 dark:border-navy-600 dark:bg-navy-700/50"
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-semibold text-gray-800 dark:text-gray-200">
                      {job.team_name}
                    </p>
                    <div className="flex items-center gap-3 mt-0.5">
                      <span className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
                        <MapPin className="w-3 h-3" />
                        {job.city}
                      </span>
                      <span className="flex items-center gap-0.5 text-xs">
                        {Array.from({ length: stars }, (_, i) => (
                          <Star
                            key={i}
                            className="w-3 h-3 fill-accent-400 text-accent-400"
                          />
                        ))}
                        {Array.from({ length: 5 - stars }, (_, i) => (
                          <Star
                            key={`e${i}`}
                            className="w-3 h-3 text-gray-300 dark:text-navy-600"
                          />
                        ))}
                      </span>
                      {job.last_league_position && (
                        <span className="text-xs text-gray-400 dark:text-gray-500">
                          {t("jobs.leaguePosition", {
                            position: job.last_league_position,
                          })}
                        </span>
                      )}
                    </div>
                  </div>
                  <button
                    onClick={() => handleApply(job)}
                    disabled={applyingTo !== null}
                    className="ml-3 shrink-0 rounded-lg bg-primary-500 px-4 py-1.5 text-xs font-heading font-bold uppercase tracking-wider text-white transition-all hover:bg-primary-600 disabled:opacity-50"
                  >
                    {applyingTo === job.team_id
                      ? t("jobs.applicationSent")
                      : t("jobs.applyButton")}
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </CardBody>
      <SwitchClubConfirmModal
        open={pendingSwitch !== null}
        currentClubName={currentClubName}
        newClubName={pendingSwitch?.team_name ?? ""}
        busy={applyingTo !== null}
        onCancel={() => setPendingSwitch(null)}
        onConfirm={handleConfirmSwitch}
      />
    </Card>
  );
}
