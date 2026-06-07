import {
  Brain,
  Cat,
  CircleDot,
  Cog,
  Crown,
  Crosshair,
  Dumbbell,
  Eye,
  Flame,
  Hand,
  Heart,
  Mountain,
  Rocket,
  Shield,
  Sparkles,
  Star,
  Target,
  Users,
  Wind,
  Zap,
} from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

type TraitCategory =
  | "physical"
  | "technical"
  | "mental"
  | "goalkeeper"
  | "special";

interface TraitMeta {
  icon: ReactNode;
  color: string;
  category: TraitCategory;
  requirements: string[];
}

const TRAIT_META: Record<string, TraitMeta> = {
  Speedster: {
    icon: <Zap className="w-3 h-3" />,
    color: "text-cyan-400 bg-cyan-500/10 ring-cyan-500/30",
    category: "physical",
    requirements: ["Pace 85+"],
  },
  Tank: {
    icon: <Dumbbell className="w-3 h-3" />,
    color: "text-orange-400 bg-orange-500/10 ring-orange-500/30",
    category: "physical",
    requirements: ["Strength 85+", "Stamina 75+"],
  },
  Agile: {
    icon: <Wind className="w-3 h-3" />,
    color: "text-teal-400 bg-teal-500/10 ring-teal-500/30",
    category: "physical",
    requirements: ["Agility 85+"],
  },
  Tireless: {
    icon: <Heart className="w-3 h-3" />,
    color: "text-green-400 bg-green-500/10 ring-green-500/30",
    category: "physical",
    requirements: ["Stamina 90+"],
  },
  Playmaker: {
    icon: <Eye className="w-3 h-3" />,
    color: "text-purple-400 bg-purple-500/10 ring-purple-500/30",
    category: "technical",
    requirements: ["Passing 80+", "Vision 80+"],
  },
  Sharpshooter: {
    icon: <Target className="w-3 h-3" />,
    color: "text-red-400 bg-red-500/10 ring-red-500/30",
    category: "technical",
    requirements: ["Shooting 85+"],
  },
  Dribbler: {
    icon: <Sparkles className="w-3 h-3" />,
    color: "text-yellow-400 bg-yellow-500/10 ring-yellow-500/30",
    category: "technical",
    requirements: ["Dribbling 85+"],
  },
  BallWinner: {
    icon: <Crosshair className="w-3 h-3" />,
    color: "text-amber-400 bg-amber-500/10 ring-amber-500/30",
    category: "technical",
    requirements: ["Tackling 80+", "Aggression 70+"],
  },
  Rock: {
    icon: <Shield className="w-3 h-3" />,
    color: "text-slate-400 bg-slate-500/10 ring-slate-500/30",
    category: "technical",
    requirements: ["Defending 85+", "Positioning 75+"],
  },
  Leader: {
    icon: <Crown className="w-3 h-3" />,
    color: "text-accent-400 bg-accent-500/10 ring-accent-500/30",
    category: "mental",
    requirements: ["Leadership 85+", "Teamwork 75+"],
  },
  CoolHead: {
    icon: <Brain className="w-3 h-3" />,
    color: "text-blue-400 bg-blue-500/10 ring-blue-500/30",
    category: "mental",
    requirements: ["Composure 85+", "Decisions 80+"],
  },
  Visionary: {
    icon: <Eye className="w-3 h-3" />,
    color: "text-indigo-400 bg-indigo-500/10 ring-indigo-500/30",
    category: "mental",
    requirements: ["Vision 85+"],
  },
  HotHead: {
    icon: <Flame className="w-3 h-3" />,
    color: "text-red-500 bg-red-500/10 ring-red-500/30",
    category: "mental",
    requirements: ["Aggression 85+", "Composure below 50"],
  },
  TeamPlayer: {
    icon: <Users className="w-3 h-3" />,
    color: "text-emerald-400 bg-emerald-500/10 ring-emerald-500/30",
    category: "mental",
    requirements: ["Teamwork 85+"],
  },
  SafeHands: {
    icon: <Hand className="w-3 h-3" />,
    color: "text-sky-400 bg-sky-500/10 ring-sky-500/30",
    category: "goalkeeper",
    requirements: ["Handling 85+"],
  },
  CatReflexes: {
    icon: <Cat className="w-3 h-3" />,
    color: "text-violet-400 bg-violet-500/10 ring-violet-500/30",
    category: "goalkeeper",
    requirements: ["Reflexes 85+"],
  },
  AerialDominance: {
    icon: <Mountain className="w-3 h-3" />,
    color: "text-sky-400 bg-sky-500/10 ring-sky-500/30",
    category: "goalkeeper",
    requirements: ["Aerial 85+"],
  },
  CompleteForward: {
    icon: <Star className="w-3 h-3" />,
    color: "text-accent-400 bg-accent-500/10 ring-accent-500/30",
    category: "special",
    requirements: [
      "Shooting 75+",
      "Dribbling 75+",
      "Pace 70+",
      "Strength 70+",
    ],
  },
  Engine: {
    icon: <Cog className="w-3 h-3" />,
    color: "text-primary-400 bg-primary-500/10 ring-primary-500/30",
    category: "special",
    requirements: ["Stamina 85+", "Pace 70+", "Teamwork 75+"],
  },
  SetPieceSpecialist: {
    icon: <CircleDot className="w-3 h-3" />,
    color: "text-lime-400 bg-lime-500/10 ring-lime-500/30",
    category: "special",
    requirements: ["Passing 80+", "Shooting 75+", "Vision 75+"],
  },
  Wonderkid: {
    icon: <Rocket className="w-3 h-3" />,
    color: "text-pink-400 bg-pink-500/10 ring-pink-500/30",
    category: "special",
    requirements: ["Age 20 or younger", "Potential 90+", "Growth room 14+"],
  },
};

function buildTraitTooltip(
  traitName: string,
  translate: (key: string) => string,
): string {
  const baseDescription = translate(`traits.${traitName}.desc`);
  const requirements = TRAIT_META[traitName]?.requirements ?? [];

  if (requirements.length === 0) {
    return baseDescription;
  }

  return `${baseDescription} | ${requirements.join(", ")}`;
}

export function getTraitMeta(
  trait: string,
): (TraitMeta & { label: string; description: string }) | null {
  const meta = TRAIT_META[trait];

  if (!meta) {
    return null;
  }

  return {
    ...meta,
    label: trait,
    description: meta.requirements.join(", "),
  };
}

export function TraitBadge({
  trait: traitName,
  size = "sm",
}: {
  trait: string;
  size?: "sm" | "xs";
}) {
  const { t } = useTranslation();
  const meta = TRAIT_META[traitName];

  if (!meta) {
    return null;
  }

  const sizeClasses =
    size === "xs"
      ? "text-[9px] px-1.5 py-0.5 gap-0.5"
      : "text-[10px] px-2 py-0.5 gap-1";
  const tooltip = buildTraitTooltip(traitName, t);

  return (
    <span
      className={`inline-flex items-center font-heading font-bold uppercase tracking-wider rounded-full ring-1 ${meta.color} ${sizeClasses}`}
      title={tooltip}
      aria-label={tooltip}
    >
      {meta.icon}
      {t(`traits.${traitName}.label`)}
    </span>
  );
}

export function TraitList({
  traits,
  size = "sm",
  max,
}: {
  traits: string[];
  size?: "sm" | "xs";
  max?: number;
}) {
  if (!traits || traits.length === 0) {
    return null;
  }

  const displayed = max ? traits.slice(0, max) : traits;
  const remaining = max && traits.length > max ? traits.length - max : 0;

  return (
    <div className="flex flex-wrap gap-1">
      {displayed.map((trait) => (
        <TraitBadge key={trait} trait={trait} size={size} />
      ))}
      {remaining > 0 ? (
        <span className="text-[10px] text-gray-500 font-heading self-center">
          +{remaining}
        </span>
      ) : null}
    </div>
  );
}

export default TraitBadge;
