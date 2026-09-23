import { Wind, Flame, Swords, Angry, ThumbsUp, Frown } from "lucide-react";

const TALK_ICON_MAP: Record<string, { icon: React.ReactNode; color: string }> = {
  calm: { icon: <Wind className="w-5 h-5" />, color: "text-sky-400" },
  motivational: { icon: <Flame className="w-5 h-5" />, color: "text-orange-400" },
  assertive: { icon: <Swords className="w-5 h-5" />, color: "text-amber-400" },
  aggressive: { icon: <Angry className="w-5 h-5" />, color: "text-red-400" },
  praise: { icon: <ThumbsUp className="w-5 h-5" />, color: "text-green-400" },
  disappointed: { icon: <Frown className="w-5 h-5" />, color: "text-gray-400" },
};

export function getTalkIcon(key: string): React.ReactNode {
  const entry = TALK_ICON_MAP[key];
  if (!entry) return null;
  return <span className={entry.color}>{entry.icon}</span>;
}

export function getTalkIconSmall(key: string): React.ReactNode {
  const map: Record<string, { icon: React.ReactNode; color: string }> = {
    calm: { icon: <Wind className="w-8 h-8" />, color: "text-sky-400" },
    motivational: { icon: <Flame className="w-8 h-8" />, color: "text-orange-400" },
    assertive: { icon: <Swords className="w-8 h-8" />, color: "text-amber-400" },
    aggressive: { icon: <Angry className="w-8 h-8" />, color: "text-red-400" },
    praise: { icon: <ThumbsUp className="w-8 h-8" />, color: "text-green-400" },
    disappointed: { icon: <Frown className="w-8 h-8" />, color: "text-gray-400" },
  };
  const entry = map[key];
  if (!entry) return null;
  return <span className={entry.color}>{entry.icon}</span>;
}
