import {
  Zap, Shield, Wind, Dumbbell, Brain, Eye, Target, Crosshair,
  Flame, Heart, Crown, Sparkles, Users,
  Hand, Cat, Mountain, Star, Cog, CircleDot, Rocket
} from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface TraitMeta {
  icon: ReactNode;
  color: string;
  category: "physical" | "technical" | "mental" | "goalkeeper" | "special";
}

const TRAIT_META: Record<string, TraitMeta> = {
  Speedster: { icon: <Zap className="w-3 h-3" />, color: "text-cyan-400 bg-cyan-500/10 ring-cyan-500/30", category: "physical" },
  Tank: { icon: <Dumbbell className="w-3 h-3" />, color: "text-orange-400 bg-orange-500/10 ring-orange-500/30", category: "physical" },
  Agile: { icon: <Wind className="w-3 h-3" />, color: "text-teal-400 bg-teal-500/10 ring-teal-500/30", category: "physical" },
  Tireless: { icon: <Heart className="w-3 h-3" />, color: "text-green-400 bg-green-500/10 ring-green-500/30", category: "physical" },
  Playmaker: { icon: <Eye className="w-3 h-3" />, color: "text-purple-400 bg-purple-500/10 ring-purple-500/30", category: "technical" },
  Sharpshooter: { icon: <Target className="w-3 h-3" />, color: "text-red-400 bg-red-500/10 ring-red-500/30", category: "technical" },
  Dribbler: { icon: <Sparkles className="w-3 h-3" />, color: "text-yellow-400 bg-yellow-500/10 ring-yellow-500/30", category: "technical" },
  BallWinner: { icon: <Crosshair className="w-3 h-3" />, color: "text-amber-400 bg-amber-500/10 ring-amber-500/30", category: "technical" },
  Rock: { icon: <Shield className="w-3 h-3" />, color: "text-slate-400 bg-slate-500/10 ring-slate-500/30", category: "technical" },
  Leader: { icon: <Crown className="w-3 h-3" />, color: "text-accent-400 bg-accent-500/10 ring-accent-500/30", category: "mental" },
  CoolHead: { icon: <Brain className="w-3 h-3" />, color: "text-blue-400 bg-blue-500/10 ring-blue-500/30", category: "mental" },
  Visionary: { icon: <Eye className="w-3 h-3" />, color: "text-indigo-400 bg-indigo-500/10 ring-indigo-500/30", category: "mental" },
  HotHead: { icon: <Flame className="w-3 h-3" />, color: "text-red-500 bg-red-500/10 ring-red-500/30", category: "mental" },
  TeamPlayer: { icon: <Users className="w-3 h-3" />, color: "text-emerald-400 bg-emerald-500/10 ring-emerald-500/30", category: "mental" },
  SafeHands: { icon: <Hand className="w-3 h-3" />, color: "text-sky-400 bg-sky-500/10 ring-sky-500/30", category: "goalkeeper" },
  CatReflexes: { icon: <Cat className="w-3 h-3" />, color: "text-violet-400 bg-violet-500/10 ring-violet-500/30", category: "goalkeeper" },
  AerialDominance: { icon: <Mountain className="w-3 h-3" />, color: "text-sky-400 bg-sky-500/10 ring-sky-500/30", category: "goalkeeper" },
  CompleteForward: { icon: <Star className="w-3 h-3" />, color: "text-accent-400 bg-accent-500/10 ring-accent-500/30", category: "special" },
  Engine: { icon: <Cog className="w-3 h-3" />, color: "text-primary-400 bg-primary-500/10 ring-primary-500/30", category: "special" },
  SetPieceSpecialist: { icon: <CircleDot className="w-3 h-3" />, color: "text-lime-400 bg-lime-500/10 ring-lime-500/30", category: "special" },
  Wonderkid: { icon: <Rocket className="w-3 h-3" />, color: "text-pink-400 bg-pink-500/10 ring-pink-500/30", category: "special" },
};

export function getTraitMeta(trait: string): (TraitMeta & { label: string; description: string }) | null {
  const meta = TRAIT_META[trait];
  if (!meta) return null;
  return { ...meta, label: trait, description: trait };
}

export function TraitBadge({ trait: traitName, size = "sm" }: { trait: string; size?: "sm" | "xs" }) {
  const { t } = useTranslation();
  const meta = TRAIT_META[traitName];
  if (!meta) return null;

  const sizeClasses = size === "xs"
    ? "text-[9px] px-1.5 py-0.5 gap-0.5"
    : "text-[10px] px-2 py-0.5 gap-1";

  return (
    <span
      className={`inline-flex items-center font-heading font-bold uppercase tracking-wider rounded-full ring-1 ${meta.color} ${sizeClasses}`}
      title={t(`traits.${traitName}.desc`)}
    >
      {meta.icon}
      {t(`traits.${traitName}.label`)}
    </span>
  );
}

export function TraitList({ traits, size = "sm", max }: { traits: string[]; size?: "sm" | "xs"; max?: number }) {
  if (!traits || traits.length === 0) return null;
  const displayed = max ? traits.slice(0, max) : traits;
  const remaining = max && traits.length > max ? traits.length - max : 0;

  return (
    <div className="flex flex-wrap gap-1">
      {displayed.map(t => <TraitBadge key={t} trait={t} size={size} />)}
      {remaining > 0 && (
        <span className="text-[10px] text-gray-500 font-heading self-center">+{remaining}</span>
      )}
    </div>
  );
}

export default TraitBadge;
