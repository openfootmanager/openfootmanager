import type { KnowledgeAnswer } from "../../hooks/useKnowledgeAnswer";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileCareerTableProps {
  career: KnowledgeAnswer<PlayerCareer | null>;
  language: string;
  /** Opens a club by its id; without it the club is plain text. */
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}

/** The career as one table. Scaffold: the real card replaces it. */
export default function PlayerProfileCareerTable(_props: PlayerProfileCareerTableProps) {
  return null;
}
