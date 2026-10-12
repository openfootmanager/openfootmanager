import { TrendingUp, ShoppingCart, Handshake } from "lucide-react";
import type { TransferAvailabilityFilter, TransferTabView } from "./TransfersTab.model";
import type { TFunction } from "i18next";
import type { TransferCollections } from "./TransfersTab.model";
import type { PlayerData } from "../../store/gameStore";
interface Input {
  collections: TransferCollections;
  myListedPlayers: PlayerData[];
  t: TFunction<"translation", undefined>;
}
export function getTransferNavigation({ collections, myListedPlayers, t }: Input) {
  const { availablePlayers, playersWithOffers, marketPlayers, loanPlayers, freeAgentPlayers } =
    collections;
  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];
  const tabs: {
    id: TransferTabView;
    label: string;
    icon: React.ReactNode;
    count: number;
  }[] = [
    {
      id: "players",
      label: t("dashboard.players"),
      icon: <TrendingUp className="w-4 h-4" />,
      count: availablePlayers.length,
    },
    {
      id: "my_list",
      label: t("transfers.myTransferList"),
      icon: <ShoppingCart className="w-4 h-4" />,
      count: myListedPlayers.length,
    },
    {
      id: "offers",
      label: t("transfers.offers"),
      icon: <Handshake className="w-4 h-4" />,
      count: playersWithOffers.length,
    },
  ];
  const availabilityFilters: {
    id: TransferAvailabilityFilter;
    label: string;
    count: number;
  }[] = [
    {
      id: "all",
      label: t("common.all"),
      count: availablePlayers.length,
    },
    {
      id: "transfer",
      label: t("transfers.transfer"),
      count: marketPlayers.length,
    },
    {
      id: "loan",
      label: t("transfers.loan"),
      count: loanPlayers.length,
    },
    {
      id: "free_agent",
      label: t("common.freeAgent"),
      count: freeAgentPlayers.length,
    },
  ];
  return { positions, tabs, availabilityFilters };
}
