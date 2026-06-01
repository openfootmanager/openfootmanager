type PlayerWithOvr = {
    ovr?: number | null;
};

export function getPlayerOvr(player: PlayerWithOvr): number {
    return player.ovr ?? 0;
}