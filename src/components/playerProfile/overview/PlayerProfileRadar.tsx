import { useTranslation } from "react-i18next";

export interface RadarAxis {
  /** The translated name, read out in full. */
  label: string;
  /** The short form drawn at the axis ("PAC"). */
  short: string;
  /** 0 to 100. */
  value: number;
  /** An axis the position cannot do without: drawn heavier. */
  key: boolean;
}

const SIZE = { width: 300, height: 200, cx: 150, cy: 100, radius: 62, labelRadius: 76 };

function pointAt(index: number, count: number, radius: number) {
  const angle = -Math.PI / 2 + (index / count) * 2 * Math.PI;
  return { x: SIZE.cx + radius * Math.cos(angle), y: SIZE.cy + radius * Math.sin(angle) };
}

const polygon = (points: { x: number; y: number }[]) =>
  points.map((point) => `${point.x.toFixed(1)},${point.y.toFixed(1)}`).join(" ");

/**
 * A player's attributes on a fixed set of axes: our own SVG on the role tokens for the shape, and
 * the labels as text on the page over it. A label inside a scaled SVG shrinks with it, below the
 * 12px floor in a narrow column; on the page it stays at the caption size wherever the chart is.
 * The picture names every value in full to a screen reader, so the visible labels are not read twice.
 */
export function PlayerProfileRadar({ axes }: { axes: RadarAxis[] }) {
  const { t } = useTranslation();
  const count = axes.length;
  const values = axes.map((axis) => `${axis.label} ${axis.value}`).join(", ");
  const ring = (scale: number) =>
    polygon(axes.map((_, index) => pointAt(index, count, SIZE.radius * scale)));
  return (
    <div className="relative mx-auto aspect-[3/2] w-full max-w-xs">
      <svg
        viewBox={`0 0 ${SIZE.width} ${SIZE.height}`}
        role="img"
        aria-label={t("playerProfile.overview.radarLabel", { values })}
        className="absolute inset-0 size-full"
      >
        <polygon points={ring(1)} className="fill-none stroke-line-strong" strokeWidth={1} />
        <polygon points={ring(0.5)} className="fill-none stroke-line" strokeWidth={1} />
        <polygon
          points={polygon(
            axes.map((axis, index) =>
              pointAt(index, count, (SIZE.radius * Math.max(0, Math.min(100, axis.value))) / 100),
            ),
          )}
          className="fill-selected-tint stroke-selected"
          strokeWidth={2}
        />
      </svg>
      {axes.map((axis, index) => {
        const at = pointAt(index, count, SIZE.labelRadius);
        const side = at.x < SIZE.cx - 4 ? "end" : at.x > SIZE.cx + 4 ? "start" : "middle";
        const shift = { end: "-translate-x-full", start: "", middle: "-translate-x-1/2" }[side];
        return (
          <span
            key={axis.label}
            aria-hidden="true"
            style={{ left: `${(at.x / SIZE.width) * 100}%`, top: `${(at.y / SIZE.height) * 100}%` }}
            className={`absolute -translate-y-1/2 whitespace-nowrap text-caption ${shift} ${
              axis.key ? "font-bold text-fg" : "text-fg-muted"
            }`}
          >
            {axis.short} {axis.value}
          </span>
        );
      })}
    </div>
  );
}
