import type { TFunction } from "i18next";
import { ArrowUpDown, GripVertical, RotateCcw, Target } from "lucide-react";
import {
  type DragEvent,
  type KeyboardEvent,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import type { RatingTier } from "../../services/squadService";
import { translatePositionAbbreviation, translatePositionLabel } from "../squad/SquadTab.helpers";
import { Badge, Button, ConditionMeter, PositionBadge, RatingChip } from "../ui";
import { moveKicker, placeInOrder, putBack } from "./shootoutOrder";
import type { EnginePlayerData } from "./types";

/** One grid for the column headings and every row, so the columns line up. */
const ROW_GRID =
  "grid grid-cols-[1.5rem_2rem_3.5rem_minmax(0,1fr)_2.25rem_2.25rem_5rem] items-center gap-2 px-3";

const FOCUS_RING =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel";

type OrderStep = "lifted" | "moved" | "dropped" | "putBack";

/** What the live region says after a step: who, and where he now stands in the order. */
function announcement(
  t: TFunction,
  step: OrderStep,
  name: string,
  index: number,
  kickers: number,
): string {
  const place = placeInOrder(index, kickers);
  if (place) {
    const at = { name, n: place.kick, k: kickers };
    switch (step) {
      case "lifted":
        return t("match.shootout.liftedKick", at);
      case "moved":
        return t("match.shootout.movedKick", at);
      case "dropped":
        return t("match.shootout.droppedKick", at);
      case "putBack":
        return t("match.shootout.putBackKick", at);
    }
  }
  switch (step) {
    case "lifted":
      return t("match.shootout.liftedSitsOut", { name });
    case "moved":
      return t("match.shootout.movedSitsOut", { name });
    case "dropped":
      return t("match.shootout.droppedSitsOut", { name });
    case "putBack":
      return t("match.shootout.putBackSitsOut", { name });
  }
}

interface OrderRowProps {
  name: string;
  player: EnginePlayerData | undefined;
  /** The kick this place takes, or null below the line. */
  kick: number | null;
  kickers: number;
  isPenaltyTaker: boolean;
  isLifted: boolean;
  isTabStop: boolean;
  tierOf: (rating: number) => RatingTier;
  grip: {
    ref: (element: HTMLButtonElement | null) => void;
    onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
    onFocus: () => void;
  };
  drag: {
    onDragStart: (event: DragEvent<HTMLLIElement>) => void;
    onDragOver: (event: DragEvent<HTMLLIElement>) => void;
    onDrop: (event: DragEvent<HTMLLIElement>) => void;
    onDragEnd: () => void;
  };
}

/** One player in the order: his kick, the grip that moves him, and what he brings to the spot. */
function OrderRow({
  name,
  player,
  kick,
  kickers,
  isPenaltyTaker,
  isLifted,
  isTabStop,
  tierOf,
  grip,
  drag,
}: OrderRowProps) {
  const { t } = useTranslation();
  return (
    <li
      draggable
      {...drag}
      className={`${ROW_GRID} h-row border-b border-l-3 border-line ${
        isLifted ? "border-l-selected bg-selected-tint" : "border-l-transparent"
      }`}
    >
      <span
        aria-hidden="true"
        className="text-right font-heading text-lead font-bold tabular-nums text-fg-muted"
      >
        {kick ?? ""}
      </span>
      <button
        type="button"
        ref={grip.ref}
        tabIndex={isTabStop ? 0 : -1}
        aria-label={
          kick === null
            ? t("match.shootout.moveSitter", { name })
            : t("match.shootout.moveKicker", { name, n: kick, k: kickers })
        }
        onKeyDown={grip.onKeyDown}
        onFocus={grip.onFocus}
        className={`grid size-8 cursor-grab place-items-center rounded-md text-fg-muted hover:bg-hover hover:text-fg ${FOCUS_RING}`}
      >
        {isLifted ? <ArrowUpDown className="size-4" /> : <GripVertical className="size-4" />}
      </button>
      <span className="flex">
        {player ? (
          <PositionBadge
            position={player.position}
            label={translatePositionAbbreviation(t, player.position)}
            name={translatePositionLabel(t, player.position)}
          />
        ) : null}
      </span>
      <span className="flex min-w-0 items-center gap-2">
        {/* A long name is cut short on a narrow window; the tooltip keeps it whole. */}
        <span
          className={`truncate font-semibold ${kick === null ? "text-fg-muted" : "text-fg"}`}
          title={name}
        >
          {name}
        </span>
        {isPenaltyTaker ? (
          <Badge>
            <Target aria-hidden="true" className="mr-1 size-3" />
            {t("match.shootout.penTaker")}
          </Badge>
        ) : null}
        {isLifted ? (
          <span className="shrink-0 rounded-md bg-selected-fill px-2 py-0.5 font-heading text-caption font-bold uppercase tracking-label text-on-brand">
            {t("match.shootout.moving")}
          </span>
        ) : null}
      </span>
      {player ? (
        <>
          <RatingChip
            value={player.shooting}
            tier={tierOf(player.shooting)}
            name={t("common.attributes.shooting")}
          />
          <RatingChip
            value={player.composure}
            tier={tierOf(player.composure)}
            name={t("common.attributes.composure")}
          />
          {/* The engine sends no condition band in a match, and the screen never makes one up. */}
          <ConditionMeter value={player.condition} status={null} />
        </>
      ) : null}
    </li>
  );
}

/**
 * The fixed line after the kickers. The rule is the separator a screen reader hears, named with how
 * many sit out and why. Not every screen reader announces a separator inside a list, so the same
 * words are also text in the entry, which names the rule; the words beside it are for the eye.
 */
function SitsOutLine({ sittingOut, kickers }: { sittingOut: number; kickers: number }) {
  const { t } = useTranslation();
  const wordsId = useId();
  return (
    <li role="none" className="flex items-center gap-3 border-b border-line bg-sunken px-3 py-1.5">
      <span id={wordsId} className="sr-only">
        {t("match.shootout.sitsOutLine", { n: sittingOut, k: kickers })}
      </span>
      <span
        aria-hidden="true"
        className="font-heading text-caption font-bold uppercase tracking-label text-warning"
      >
        {t("match.shootout.sitsOut")}
      </span>
      <hr
        aria-labelledby={wordsId}
        className="min-w-4 flex-1 border-0 border-t-2 border-dashed border-warning-solid"
      />
      <span aria-hidden="true" className="text-caption text-fg-muted">
        {t("match.shootout.sitsOutNote", { k: kickers })}
      </span>
    </li>
  );
}

/** The columns' names, for the eye: every value in a row names itself to a screen reader. */
function ColumnHeadings() {
  const { t } = useTranslation();
  const cell = "text-caption font-medium text-fg-subtle";
  return (
    <div aria-hidden="true" className={`${ROW_GRID} h-8 border-b border-line-strong`}>
      <span />
      <span />
      <span className={cell}>{t("squad.pos")}</span>
      <span className={cell}>{t("common.player")}</span>
      <span className={cell}>{t("match.shootout.columnShooting")}</span>
      <span className={cell}>{t("match.shootout.columnComposure")}</span>
      <span className={cell}>{t("common.condition")}</span>
    </div>
  );
}

interface ShootoutOrderPickerProps {
  /** Every eligible player's id, in the order being set. */
  order: readonly string[];
  /** The order the engine first gave, which Use auto order brings back. */
  autoOrder: readonly string[];
  /** How many of the order kick; the line sits after them and never moves. */
  kickers: number;
  /** The user's players on the pitch and the bench, by id. */
  players: ReadonlyMap<string, EnginePlayerData>;
  penaltyTakerId: string | null;
  /** Whether the order is on its way to the engine. */
  starting: boolean;
  /** The engine's refusal, in words; null when there is none. */
  error: string | null;
  onOrderChange: (order: readonly string[]) => void;
  onStart: () => void;
}

/**
 * Lifting and moving players in the order, by keyboard or by drag: which player is lifted, which
 * grip is the list's one tab stop, and what the live region last said.
 */
function useOrderMoves({
  order,
  autoOrder,
  kickers,
  players,
  onOrderChange,
}: Pick<
  ShootoutOrderPickerProps,
  "order" | "autoOrder" | "kickers" | "players" | "onOrderChange"
>) {
  const { t } = useTranslation();
  const [lifted, setLifted] = useState<{ id: string; origin: number } | null>(null);
  const [activeId, setActiveId] = useState(order[0] ?? "");
  const [message, setMessage] = useState("");
  const grips = useRef(new Map<string, HTMLButtonElement>());
  const refocus = useRef(false);
  const dragged = useRef<string | null>(null);

  const nameOf = (id: string) => players.get(id)?.name ?? id;
  const tabStop = order.includes(activeId) ? activeId : order[0];

  // A reordered row is moved in the DOM, which drops the focus; the moved player keeps it.
  useLayoutEffect(() => {
    if (!refocus.current) return;
    refocus.current = false;
    grips.current.get(activeId)?.focus();
  }, [activeId, order]);

  const focusOn = (id: string) => {
    setActiveId(id);
    refocus.current = true;
  };
  const say = (step: OrderStep, id: string, index: number) =>
    setMessage(announcement(t, step, nameOf(id), index, kickers));

  const moveOrFocus = (id: string, index: number, to: number) => {
    if (lifted?.id !== id) {
      const neighbour = order[to];
      if (neighbour !== undefined) focusOn(neighbour);
      return;
    }
    const next = moveKicker(order, index, to);
    if (next === order) return;
    onOrderChange(next);
    say("moved", id, to);
    focusOn(id);
  };

  const handleKeyDown = (id: string) => (event: KeyboardEvent<HTMLButtonElement>) => {
    const index = order.indexOf(id);
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      moveOrFocus(id, index, index + (event.key === "ArrowUp" ? -1 : 1));
    } else if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      if (lifted?.id === id) {
        setLifted(null);
        say("dropped", id, index);
      } else {
        setLifted({ id, origin: index });
        say("lifted", id, index);
      }
    } else if (event.key === "Escape" && lifted?.id === id) {
      event.preventDefault();
      onOrderChange(putBack(order, id, lifted.origin));
      setLifted(null);
      say("putBack", id, lifted.origin);
      focusOn(id);
    }
  };

  const dragHandlers = (id: string, index: number) => ({
    onDragStart: (event: DragEvent<HTMLLIElement>) => {
      dragged.current = id;
      setLifted(null);
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", id);
    },
    onDragOver: (event: DragEvent<HTMLLIElement>) => {
      if (dragged.current === null) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
    },
    onDrop: (event: DragEvent<HTMLLIElement>) => {
      event.preventDefault();
      const from = dragged.current;
      dragged.current = null;
      if (from === null) return;
      const next = moveKicker(order, order.indexOf(from), index);
      if (next === order) return;
      onOrderChange(next);
      say("dropped", from, index);
      setActiveId(from);
    },
    onDragEnd: () => {
      dragged.current = null;
    },
  });

  const restoreAutoOrder = () => {
    onOrderChange(autoOrder);
    setLifted(null);
    setMessage(t("match.shootout.autoOrderRestored"));
  };

  const gripRef = (id: string) => (element: HTMLButtonElement | null) => {
    if (element) grips.current.set(id, element);
    else grips.current.delete(id);
  };

  return {
    lifted,
    tabStop,
    message,
    nameOf,
    gripRef,
    handleKeyDown,
    dragHandlers,
    restoreAutoOrder,
    takeFocus: setActiveId,
  };
}

/**
 * Step 1 of the shootout: the manager's penalty order, before the first kick. The list is one tab
 * stop: the arrow keys move the focus, Space or Enter lifts a player, the arrows then carry him and
 * Space or Enter drops him, and Escape puts him back. A row can also be dragged. The line after the
 * kickers is fixed: moving a player across it swaps who kicks and who sits out.
 */
export function ShootoutOrderPicker({
  order,
  autoOrder,
  kickers,
  players,
  penaltyTakerId,
  starting,
  error,
  onOrderChange,
  onStart,
}: ShootoutOrderPickerProps) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  const moves = useOrderMoves({ order, autoOrder, kickers, players, onOrderChange });

  const rows = order.map((id, index) => (
    <OrderRow
      key={id}
      name={moves.nameOf(id)}
      player={players.get(id)}
      kick={placeInOrder(index, kickers)?.kick ?? null}
      kickers={kickers}
      isPenaltyTaker={penaltyTakerId === id}
      isLifted={moves.lifted?.id === id}
      isTabStop={moves.tabStop === id}
      tierOf={tierOf}
      grip={{
        ref: moves.gripRef(id),
        onKeyDown: moves.handleKeyDown(id),
        onFocus: () => moves.takeFocus(id),
      }}
      drag={moves.dragHandlers(id, index)}
    />
  ));
  if (kickers < order.length) {
    rows.splice(
      kickers,
      0,
      <SitsOutLine key="sits-out" sittingOut={order.length - kickers} kickers={kickers} />,
    );
  }

  return (
    <section
      aria-labelledby="shootout-order-heading"
      className="flex flex-col overflow-clip rounded-xl bg-panel shadow-sm"
    >
      <header className="flex min-h-panel-header flex-wrap items-center gap-3 border-b border-line px-4 py-2">
        <h2 id="shootout-order-heading" className="font-heading text-heading font-bold text-fg">
          {t("match.shootout.orderHeading")}
        </h2>
        <span className="rounded-full bg-info-tint px-2 py-0.5 font-heading text-caption font-bold uppercase tracking-label text-info">
          {t("match.shootout.orderStep")}
        </span>
        <Button
          variant="ghost"
          size="sm"
          icon={<RotateCcw />}
          onClick={moves.restoreAutoOrder}
          className="ml-auto"
        >
          {t("match.shootout.useAutoOrder")}
        </Button>
      </header>
      <p id="shootout-order-hint" className="px-4 py-2 text-caption text-fg-muted">
        {t("match.shootout.orderHint")}
      </p>
      <div className="overflow-x-auto">
        <div className="min-w-[34rem]">
          <ColumnHeadings />
          <ol aria-labelledby="shootout-order-heading" aria-describedby="shootout-order-hint">
            {rows}
          </ol>
        </div>
      </div>
      <footer className="sticky bottom-0 flex flex-wrap items-center gap-3 border-t border-line bg-panel px-4 py-3">
        <p role="status" className="min-w-0 flex-1 text-caption text-fg-muted">
          {moves.message}
        </p>
        {error ? (
          <p
            role="alert"
            className="w-full rounded-lg bg-danger-tint px-3 py-2 text-body font-semibold text-danger"
          >
            {error}
          </p>
        ) : null}
        <Button onClick={onStart} disabled={starting} className="ml-auto">
          {t("match.shootout.startShootout")}
        </Button>
      </footer>
    </section>
  );
}
