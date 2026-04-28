import { useState } from "react";
import { MatchSnapshot } from "./types";
import { Bug } from "lucide-react";

interface DebugOverlayProps {
  snapshot: MatchSnapshot;
}

export default function DebugOverlay({ snapshot }: DebugOverlayProps) {
  const [isOpen, setIsOpen] = useState(false);

  if (!isOpen) {
    return (
      <button
        onClick={() => setIsOpen(true)}
        className="fixed bottom-4 left-4 z-50 p-2 bg-yellow-500/90 hover:bg-yellow-500 text-white rounded-full shadow-lg transition-all"
        title="Debug Overlay"
      >
        <Bug className="w-5 h-5" />
      </button>
    );
  }

  const userTeam = snapshot.possession === "Home" ? snapshot.home_team : snapshot.away_team;
  const oppTeam = snapshot.possession === "Home" ? snapshot.away_team : snapshot.home_team;

  return (
    <div className="fixed bottom-4 left-4 z-50 w-80 max-h-[70vh] overflow-auto bg-gray-900/95 text-green-400 rounded-lg shadow-2xl border border-green-500/30 font-mono text-xs">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-green-500/30 bg-green-500/10">
        <span className="font-bold uppercase tracking-wider">Debug</span>
        <button
          onClick={() => setIsOpen(false)}
          className="text-green-400 hover:text-white transition-colors"
        >
          ✕
        </button>
      </div>

      {/* Match State */}
      <div className="p-3 border-b border-green-500/20">
        <div className="grid grid-cols-2 gap-1">
          <span className="text-gray-400">Phase:</span>
          <span>{snapshot.phase}</span>
          <span className="text-gray-400">Minute:</span>
          <span>{snapshot.current_minute}'</span>
          <span className="text-gray-400">Ball Zone:</span>
          <span className="text-yellow-300">{snapshot.ball_zone}</span>
          <span className="text-gray-400">Possession:</span>
          <span>{snapshot.possession}</span>
          <span className="text-gray-400">Score:</span>
          <span>{snapshot.home_score} - {snapshot.away_score}</span>
        </div>
      </div>

      {/* Team with Possession */}
      <div className="p-3 border-b border-green-500/20">
        <div className="font-bold mb-2 text-white">
          {userTeam.name} (ATT) — {userTeam.formation}
        </div>
        <div className="space-y-1">
          {userTeam.players.map((p) => (
            <div key={p.id} className="flex items-center gap-2">
              <span className="text-gray-500 w-6">{p.position.substring(0, 3)}</span>
              <span className="text-yellow-200">{p.natural_position}</span>
              <span className="truncate flex-1">{p.name}</span>
              <span className="text-gray-500">{p.condition}%</span>
            </div>
          ))}
        </div>
      </div>

      {/* Opponent Team */}
      <div className="p-3">
        <div className="font-bold mb-2 text-white">
          {oppTeam.name} (DEF) — {oppTeam.formation}
        </div>
        <div className="space-y-1">
          {oppTeam.players.map((p) => (
            <div key={p.id} className="flex items-center gap-2">
              <span className="text-gray-500 w-6">{p.position.substring(0, 3)}</span>
              <span className="text-yellow-200">{p.natural_position}</span>
              <span className="truncate flex-1">{p.name}</span>
              <span className="text-gray-500">{p.condition}%</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
