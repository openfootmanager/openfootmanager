import { AlertTriangle } from "lucide-react";
import { resolvePlayerInjuryName } from "./PlayerProfile.helpers";
import { Card, CardBody } from "../ui";
import type { TOptions } from "i18next";

type TranslateFn = (key: string, options?: TOptions) => string;

interface PlayerProfileInjuryBannerProps {
  injury: {
    name: string;
    days_remaining: number;
  };
  t: TranslateFn;
}

export default function PlayerProfileInjuryBanner({ injury, t }: PlayerProfileInjuryBannerProps) {
  return (
    <Card accent="danger" className="mb-5">
      <CardBody>
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-danger-tint flex items-center justify-center">
            <AlertTriangle className="w-5 h-5 text-danger" />
          </div>
          <div>
            <p className="font-semibold text-body text-danger">
              {resolvePlayerInjuryName(injury.name, t)}
            </p>
            <p className="text-caption text-fg-muted">
              {t("playerProfile.daysRemaining", {
                count: injury.days_remaining,
              })}
            </p>
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
