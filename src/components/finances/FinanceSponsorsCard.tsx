import { Card, CardHeader, CardBody, Button } from "../ui";
import { formatExactMoney } from "../../lib/helpers";

import type { resolveMessage } from "../../utils/backendI18n";
import { isChooseOptionAction } from "./FinancesTab.helpers";

import type { FinanceViewState } from "./useFinanceView";

export function FinanceSponsorsCard({
  state,
}: {
  state: Pick<
    FinanceViewState,
    | "t"
    | "activeSponsorship"
    | "sponsorPitchPreviewText"
    | "handleRequestSponsorPitch"
    | "actionLoading"
    | "canRequestSponsorPitch"
    | "sponsorPitchDisabledReason"
    | "sponsorPitchFeedback"
    | "marketingCampaignPreviewText"
    | "handleRequestMarketingCampaign"
    | "canRequestMarketingCampaign"
    | "marketingCampaignDisabledReason"
    | "marketingCampaignFeedback"
    | "sponsorOffers"
    | "handleSponsorOption"
    | "answeringOfferId"
    | "sponsorOfferError"
  >;
}) {
  const {
    t,
    activeSponsorship,
    sponsorPitchPreviewText,
    handleRequestSponsorPitch,
    actionLoading,
    canRequestSponsorPitch,
    sponsorPitchDisabledReason,
    sponsorPitchFeedback,
    marketingCampaignPreviewText,
    handleRequestMarketingCampaign,
    canRequestMarketingCampaign,
    marketingCampaignDisabledReason,
    marketingCampaignFeedback,
    sponsorOffers,
  } = state;
  return (
    <Card className="lg:col-span-3">
      <CardHeader>{t("finances.sponsors")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="rounded-xl border border-line bg-sunken p-4 space-y-2">
            <p className="text-caption font-heading font-bold text-fg-muted">
              {t("finances.activeSponsor")}
            </p>
            {activeSponsorship ? (
              <>
                <h3 className="font-heading font-bold text-body text-fg uppercase tracking-wide">
                  {activeSponsorship.sponsor_name}
                </h3>
                <p className="text-body text-fg-muted">
                  {t("finances.sponsorWeeklyValue", {
                    amount: formatExactMoney(activeSponsorship.base_value),
                  })}
                </p>
                <p className="text-body text-fg-muted">
                  {t("finances.sponsorRemainingWeeks", {
                    count: activeSponsorship.remaining_weeks,
                  })}
                </p>
              </>
            ) : (
              <p className="text-body text-fg-muted">{t("finances.noActiveSponsor")}</p>
            )}
          </div>

          <div className="rounded-xl border border-line bg-sunken p-4 space-y-3">
            <p className="text-caption font-heading font-bold text-fg-muted">
              {t("finances.pendingSponsorOffers")}
            </p>
            <div className="rounded-lg border border-line bg-panel p-4 space-y-3">
              <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                <div className="space-y-1">
                  <h3 className="font-semibold text-body text-fg">{t("finances.pitchSponsor")}</h3>
                  <p className="text-body text-fg-muted">{t("finances.sponsorPitchDescription")}</p>
                  {sponsorPitchPreviewText ? (
                    <p className="text-caption text-fg-muted">{sponsorPitchPreviewText}</p>
                  ) : null}
                </div>
                <Button
                  size="sm"
                  onClick={() => void handleRequestSponsorPitch()}
                  disabled={actionLoading === "sponsor-pitch" || !canRequestSponsorPitch}
                >
                  {t("finances.pitchSponsor")}
                </Button>
              </div>
              {sponsorPitchDisabledReason ? (
                <p className="text-caption text-fg-muted">{sponsorPitchDisabledReason}</p>
              ) : null}
              {sponsorPitchFeedback ? (
                <p
                  className={
                    sponsorPitchFeedback.tone === "error"
                      ? "text-body text-danger"
                      : "text-body text-positive"
                  }
                >
                  {sponsorPitchFeedback.text}
                </p>
              ) : null}
            </div>
            <div className="rounded-lg border border-line bg-panel p-4 space-y-3">
              <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                <div className="space-y-1">
                  <h3 className="font-semibold text-body text-fg">
                    {t("finances.marketingCampaign")}
                  </h3>
                  <p className="text-body text-fg-muted">
                    {t("finances.marketingCampaignDescription")}
                  </p>
                  {marketingCampaignPreviewText ? (
                    <p className="text-caption text-fg-muted">{marketingCampaignPreviewText}</p>
                  ) : null}
                </div>
                <Button
                  size="sm"
                  onClick={() => void handleRequestMarketingCampaign()}
                  disabled={actionLoading === "marketing-campaign" || !canRequestMarketingCampaign}
                >
                  {t("finances.launchMarketingCampaign")}
                </Button>
              </div>
              {marketingCampaignDisabledReason ? (
                <p className="text-caption text-fg-muted">{marketingCampaignDisabledReason}</p>
              ) : null}
              {marketingCampaignFeedback ? (
                <p
                  className={
                    marketingCampaignFeedback.tone === "error"
                      ? "text-body text-danger"
                      : "text-body text-positive"
                  }
                >
                  {marketingCampaignFeedback.text}
                </p>
              ) : null}
            </div>
            {sponsorOffers.length > 0 ? (
              sponsorOffers.map((message) => (
                <FinanceSponsorOffer key={message.id} message={message} state={state} />
              ))
            ) : (
              <p className="text-body text-fg-muted">{t("finances.noPendingSponsorOffers")}</p>
            )}
          </div>
        </div>
      </CardBody>
    </Card>
  );
}

function FinanceSponsorOffer({
  message,
  state,
}: {
  message: ReturnType<typeof resolveMessage>;
  state: Pick<FinanceViewState, "answeringOfferId" | "sponsorOfferError" | "handleSponsorOption">;
}) {
  const { answeringOfferId, sponsorOfferError, handleSponsorOption } = state;

  const sponsorAction = message.actions.find(
    (action) => !action.resolved && isChooseOptionAction(action.action_type),
  );

  if (!sponsorAction || !isChooseOptionAction(sponsorAction.action_type)) {
    return null;
  }

  return (
    <div key={message.id} className="rounded-lg border border-line bg-panel p-4 space-y-3">
      <div className="space-y-1">
        <h3 className="font-semibold text-body text-fg">{message.subject}</h3>
        <p className="text-body text-fg-muted">{message.body}</p>
      </div>
      {sponsorOfferError?.messageId === message.id ? (
        <p role="alert" className="text-body text-danger">
          {sponsorOfferError.text}
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        {sponsorAction.action_type.ChooseOption.options.map((option) => {
          return (
            <div
              key={option.id}
              className="min-w-55 flex-1 rounded-lg border border-line bg-sunken p-3 space-y-2"
            >
              <p className="text-caption text-fg-muted">{option.description}</p>
              <Button
                // An answer in flight holds every sponsor answer: the opposite one must not be sent
                // too, whatever else the tab is doing.
                disabled={answeringOfferId !== null}
                onClick={() => void handleSponsorOption(message.id, sponsorAction.id, option.id)}
                size="sm"
                variant={option.id === "decline" ? "outline" : "primary"}
              >
                {option.label}
              </Button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
