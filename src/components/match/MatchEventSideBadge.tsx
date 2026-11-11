import { Badge } from "../ui";

interface MatchEventSideBadgeProps {
  side: "Home" | "Away";
  homeTeam: { short_name: string };
  awayTeam: { short_name: string };
}

/** Which club an event belongs to, by the same short name the score header shows. */
export default function MatchEventSideBadge({
  side,
  homeTeam,
  awayTeam,
}: MatchEventSideBadgeProps) {
  return (
    <Badge variant={side === "Home" ? "primary" : "accent"} size="sm">
      {(side === "Home" ? homeTeam : awayTeam).short_name}
    </Badge>
  );
}
