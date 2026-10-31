import { FastForward, Globe, Play, SkipForward, Snail, Trophy } from "lucide-react";
import { type ReactNode, useState } from "react";

import {
  Badge,
  Button,
  Card,
  CardBody,
  CardHeader,
  Checkbox,
  CountryFlag,
  DataTable,
  type DataTableColumn,
  DatePicker,
  Dialog,
  DialogCloseButton,
  InjuryBadge,
  UnreadBadge,
  JerseyIcon,
  PageToolbar,
  PitchToken,
  PlayerAvatar,
  PositionBadge,
  PositionBadges,
  ProgressBar,
  RatingChip,
  SegmentedControl,
  Select,
  type SortDirection,
  Switch,
  TONE_TEXT,
  TabPanel,
  Tabs,
  TeamLocation,
  TeamLogo,
  ThemeToggle,
} from "../../components/ui";
import { ChartContainer } from "../../components/ui/charts/ChartContainer";
import type { StyleGuideOptions } from "./styleGuideOptions";

/*
 * Every primitive in src/components/ui, in every state that changes how it looks, drawn from
 * fixtures with no backend. Dev only, and English only: no player ever sees this page.
 * Screenshots of it are the before/after evidence for a primitive PR.
 */

// Full custom-property names on purpose: Tailwind v4 only emits a theme variable whose name it finds
// in the source, so a name built from fragments would resolve to nothing.
const SURFACES = [
  "--color-page",
  "--color-panel",
  "--color-sunken",
  "--color-hover",
  "--color-chrome",
  "--color-overlay",
];
const TEXT_AND_LINES = [
  "--color-fg",
  "--color-fg-muted",
  "--color-fg-subtle",
  "--color-line",
  "--color-line-strong",
  "--color-focus",
  "--color-on-brand",
  "--color-chrome-fg",
  "--color-chrome-fg-muted",
];
const STATUSES = [
  ["--color-positive", "--color-positive-tint", "--color-positive-solid"],
  ["--color-warning", "--color-warning-tint", "--color-warning-solid"],
  ["--color-danger", "--color-danger-tint", "--color-danger-solid"],
  ["--color-info", "--color-info-tint", "--color-info-solid"],
].flat();
const TONES = [
  ["--color-tone-top", "--color-tone-top-tint", "--color-tone-top-solid"],
  ["--color-tone-good", "--color-tone-good-tint", "--color-tone-good-solid"],
  ["--color-tone-fair", "--color-tone-fair-tint", "--color-tone-fair-solid"],
  ["--color-tone-low", "--color-tone-low-tint", "--color-tone-low-solid"],
  ["--color-tone-bad", "--color-tone-bad-tint", "--color-tone-bad-solid"],
].flat();
const BRAND = [
  ["--color-primary-50", "--color-primary-100", "--color-primary-200", "--color-primary-300"],
  ["--color-primary-400", "--color-primary-500", "--color-primary-600", "--color-primary-700"],
  ["--color-primary-800", "--color-primary-900"],
  ["--color-accent-50", "--color-accent-100", "--color-accent-200", "--color-accent-300"],
  ["--color-accent-400", "--color-accent-500", "--color-accent-600", "--color-accent-700"],
  ["--color-accent-800", "--color-accent-900"],
  ["--color-navy-900", "--color-navy-800", "--color-navy-700", "--color-navy-600"],
  ["--color-success-400", "--color-success-500", "--color-success-600"],
].flat();
const DATA_AND_PITCH = [
  "--color-series-1",
  "--color-series-2",
  "--color-kit-home-fallback",
  "--color-kit-away-fallback",
  "--color-turf-near",
  "--color-turf-far",
  "--color-pitch-line",
  "--color-pitch-press",
  "--color-pitch-defensive-line",
];
// Static classes, not an inline size from the variable: the guide follows the type rule it shows.
const TYPE_ROLES = [
  { token: "--text-caption", className: "text-caption" },
  { token: "--text-body", className: "text-body" },
  { token: "--text-lead", className: "text-lead" },
  { token: "--text-heading", className: "text-heading" },
  { token: "--text-title", className: "text-title" },
  { token: "--text-display", className: "text-display" },
];
const HEIGHTS = [
  { token: "--spacing-row", className: "h-row", use: "a table or list row" },
  { token: "--spacing-panel-header", className: "h-panel-header", use: "a panel's header" },
  { token: "--spacing-toolbar", className: "h-toolbar", use: "a page toolbar" },
];
const WIDTHS = [
  { token: "--container-page", use: "the page width (max-w-page)" },
  { token: "--spacing-sidebar-rail", use: "the sidebar's icon rail (w-sidebar-rail)" },
  { token: "--spacing-sidebar-compact", use: "the open sidebar, Compact (w-sidebar-compact)" },
  { token: "--spacing-sidebar-standard", use: "the open sidebar, Standard (w-sidebar-standard)" },
  { token: "--spacing-sidebar-wide", use: "the open sidebar, Wide (w-sidebar-wide)" },
  {
    token: "--container-split",
    use: "two panels side by side above this container width (@split)",
  },
];
const FONTS = [
  { token: "--font-heading", className: "font-heading uppercase tracking-label" },
  { token: "--font-sans", className: "font-sans" },
];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3" aria-label={title}>
      <h2 className="font-heading text-heading uppercase tracking-label text-fg">{title}</h2>
      {children}
    </section>
  );
}

function Swatch({ token }: { token: string }) {
  return (
    <div className="overflow-hidden rounded-lg border border-line bg-panel">
      <div className="h-10 border-b border-line" style={{ background: `var(${token})` }} />
      <p className="px-2 py-1 text-caption text-fg">{token.replace("--color-", "")}</p>
    </div>
  );
}

function Swatches({ tokens }: { tokens: string[] }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-2">
      {tokens.map((token) => (
        <Swatch key={token} token={token} />
      ))}
    </div>
  );
}

function Row({ children }: { children: ReactNode }) {
  return <div className="flex flex-wrap items-center gap-3">{children}</div>;
}

function TokensSection() {
  return (
    <Section title="Tokens">
      <Swatches tokens={SURFACES} />
      <Swatches tokens={TEXT_AND_LINES} />
      <Swatches tokens={STATUSES} />
      <Swatches tokens={TONES} />
      <Swatches tokens={BRAND} />
      <Swatches tokens={DATA_AND_PITCH} />
      <div className="flex flex-col gap-1 rounded-lg border border-line bg-panel p-4">
        {TYPE_ROLES.map(({ token, className }) => (
          <p key={token} className={`${className} text-fg`}>
            {token.replace("--text-", "")}: Callum Fraser · Central midfielder · 77
          </p>
        ))}
      </div>
      <LayoutTokens />
    </Section>
  );
}

/** Heights drawn at their size (they shrink under compact density); widths, fonts and tracking named. */
function LayoutTokens() {
  return (
    <div className="grid gap-2 rounded-lg border border-line bg-panel p-4 text-caption text-fg-muted">
      {HEIGHTS.map(({ token, className, use }) => (
        <div key={token} className="flex items-center gap-3">
          <div
            className={`${className} w-16 shrink-0 rounded-sm bg-sunken ring-1 ring-inset ring-line`}
          />
          <span>
            <span className="text-fg">{token.replace("--", "")}</span>: {use}
          </span>
        </div>
      ))}
      {WIDTHS.map(({ token, use }) => (
        <p key={token}>
          <span className="text-fg">{token.replace("--", "")}</span>: {use}
        </p>
      ))}
      {FONTS.map(({ token, className }) => (
        <p key={token} className={`${className} text-body text-fg`}>
          {token.replace("--", "")}: Callum Fraser · 77
        </p>
      ))}
      <p>
        <span className="text-fg">{"--tracking-label".replace("--", "")}</span>: the letter spacing
        of uppercase headings, buttons, badges and tabs
      </p>
    </div>
  );
}

function ControlsSection() {
  const [checked, setChecked] = useState(true);
  const [date, setDate] = useState("1997-09-12");
  return (
    <Section title="Controls">
      <Row>
        {(["primary", "accent", "ghost", "outline"] as const).map((variant) => (
          <Button key={variant} variant={variant}>
            {variant}
          </Button>
        ))}
        <Button size="sm" icon={<Trophy />}>
          small
        </Button>
        <Button size="lg">large</Button>
        <Button disabled>disabled</Button>
        <ThemeToggle />
      </Row>
      <Row>
        {(["default", "subtle", "muted", "highlighted", "placeholder"] as const).map((variant) => (
          <Select
            key={variant}
            variant={variant}
            aria-label={`Select ${variant}`}
            defaultValue="en"
          >
            <option value="en">{variant}</option>
            <option value="de">Deutsch</option>
          </Select>
        ))}
        {(["2xs", "xs", "sm", "md"] as const).map((size) => (
          <Select
            key={size}
            selectSize={size}
            icon={<Globe />}
            aria-label={`Select size ${size}`}
            defaultValue="a"
          >
            <option value="a">size {size}</option>
          </Select>
        ))}
      </Row>
      <Row>
        <Checkbox checked={checked} onChange={() => setChecked(!checked)} aria-label="Checked" />
        <Checkbox checked={false} onChange={() => undefined} aria-label="Unchecked" />
        <Checkbox checked={false} indeterminate onChange={() => undefined} aria-label="Mixed" />
        <Checkbox checked onChange={() => undefined} disabled aria-label="Disabled" />
      </Row>
      <Row>
        <Switch label="Switch" checked={checked} onChange={setChecked} />
        <Switch label="Switch, off" checked={false} onChange={() => undefined} />
        <Switch label="Switch, disabled on" checked disabled onChange={() => undefined} />
        <Switch label="Switch, disabled off" checked={false} disabled onChange={() => undefined} />
      </Row>
      <div className="grid max-w-md gap-3">
        <DatePicker value={date} onChange={setDate} />
        <DatePicker value="" onChange={() => undefined} error />
      </div>
    </Section>
  );
}

function DisplaySection() {
  return (
    <Section title="Display">
      <Row>
        {(["primary", "accent", "success", "danger", "neutral"] as const).map((variant) => (
          <Badge key={variant} variant={variant}>
            {variant}
          </Badge>
        ))}
        <Badge size="md">medium</Badge>
        <InjuryBadge injury={{ name: "Hamstring strain", days_remaining: 4 }} />
        <InjuryBadge injury={{ name: "Metatarsal fracture", days_remaining: 42 }} />
        <UnreadBadge count={3} />
        <UnreadBadge count={42} />
        <UnreadBadge count={120} />
      </Row>
      <Row>
        {/* The backend's tiers (S, A, B and the rest), at the row's size and a card's. */}
        {(
          [
            [88, "S"],
            [79, "A"],
            [71, "B"],
            [58, "Rest"],
          ] as const
        ).map(([value, tier]) => (
          <RatingChip key={tier} value={value} tier={tier} />
        ))}
        {(
          [
            [88, "S"],
            [58, "Rest"],
          ] as const
        ).map(([value, tier]) => (
          <RatingChip key={`md-${tier}`} value={value} tier={tier} size="md" />
        ))}
      </Row>
      <div className="grid max-w-md gap-2">
        {(["auto", "primary", "accent", "success", "danger", "muted"] as const).map(
          (variant, i) => (
            <ProgressBar key={variant} value={95 - i * 17} variant={variant} showLabel />
          ),
        )}
      </div>
      {/* The good-to-bad scale the backend rates values on: a tone fills the bar, its word says it. */}
      <div className="grid max-w-md gap-2">
        {(
          [
            ["good", "Fresh", 90],
            ["fair", "Tiring", 60],
            ["low", "Exhausted", 32],
            ["bad", "Critical", 15],
          ] as const
        ).map(([tone, word, value]) => (
          <div key={tone} className="flex items-center gap-3">
            <span className={`w-20 text-caption font-semibold ${TONE_TEXT[tone]}`}>{word}</span>
            <ProgressBar value={value} tone={tone} showLabel className="flex-1" />
          </div>
        ))}
      </div>
      <Row>
        <CountryFlag code="BE" />
        <CountryFlag code="ENG" />
        <CountryFlag code="NZ" />
        <TeamLocation city="Brussels" countryCode="BE" />
        <TeamLogo
          team={{
            name: "Brussels FC",
            short_name: "BRU",
            colors: { primary: "#15803d", secondary: "#ffffff" },
          }}
        />
        <TeamLogo
          team={{
            name: "Sporting Waregem",
            short_name: "SPA",
            colors: { primary: "#1e3a8a", secondary: "#dc2626" },
          }}
        />
        <PlayerAvatar
          player={{ full_name: "Callum Fraser", match_name: "Fraser" }}
          enableRuntimePortrait={false}
        />
        {(["Solid", "Stripes", "Hoops", "HalfAndHalf", "Diagonal"] as const).map((pattern) => (
          <JerseyIcon
            key={pattern}
            primaryColor="#15803d"
            secondaryColor="#ffffff"
            pattern={pattern}
            number={10}
          />
        ))}
      </Row>
    </Section>
  );
}

function CardsSection() {
  return (
    <Section title="Cards">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(16rem,1fr))] gap-3">
        {(["none", "primary", "accent", "success", "danger"] as const).map((accent) => (
          <Card key={accent} accent={accent}>
            <CardHeader
              action={
                <Button size="sm" variant="ghost">
                  view
                </Button>
              }
            >
              {accent}
            </CardHeader>
            <CardBody>
              <p className="text-body text-fg-muted">Expires 30 Jun 2028 · €22K a week</p>
            </CardBody>
          </Card>
        ))}
        <ChartContainer isEmpty height={120} />
      </div>
    </Section>
  );
}

const TABLE_FIXTURES = [
  { id: "t1", name: "Anderson", age: 31, morale: 72, starter: true, injured: false },
  { id: "t2", name: "Navarro", age: 33, morale: 55, starter: true, injured: false },
  { id: "t3", name: "Johnston", age: 29, morale: 61, starter: false, injured: true },
  { id: "t4", name: "Graham", age: 34, morale: 80, starter: false, injured: false },
];
type TableFixture = (typeof TABLE_FIXTURES)[number];

const TABLE_COLUMNS: DataTableColumn<TableFixture>[] = [
  { key: "name", header: "Name", cell: (row) => row.name, primary: true, sortable: true },
  { key: "age", header: "Age", cell: (row) => row.age, numeric: true, sortable: true },
  // Folds into the row's details in a table narrower than 48rem.
  { key: "morale", header: "Morale", cell: (row) => row.morale, numeric: true, drawer: true },
];

/** The panel a page puts its table in: DataTable draws none of its own. */
function TablePanel({ children }: { children: ReactNode }) {
  return <div className="overflow-hidden rounded-lg border border-line bg-panel">{children}</div>;
}

function TablesSection() {
  const [sort, setSort] = useState<{ key: string; direction: SortDirection }>({
    key: "age",
    direction: "desc",
  });
  const sorted = [...TABLE_FIXTURES].sort((a, b) => {
    const order = sort.key === "name" ? a.name.localeCompare(b.name) : a.age - b.age;
    return sort.direction === "asc" ? order : -order;
  });
  return (
    <Section title="Tables">
      <div className="grid gap-4">
        <TablePanel>
          <DataTable
            label="Squad (sortable; starters and an injury accented; morale folds when narrow)"
            columns={TABLE_COLUMNS}
            rows={sorted}
            rowKey={(row) => row.id}
            rowName={(row) => row.name}
            sort={sort}
            onSortChange={(key) =>
              setSort((current) => ({
                key,
                direction: current.key === key && current.direction === "desc" ? "asc" : "desc",
              }))
            }
            onOpen={() => undefined}
            rowAccent={(row) => (row.injured ? "danger" : row.starter ? "positive" : null)}
            rowAction={{
              header: "Actions",
              cell: (row) => (
                <Button size="sm" variant="ghost" aria-label={`Menu for ${row.name}`}>
                  ⋯
                </Button>
              ),
            }}
          />
        </TablePanel>
        <div className="max-w-md">
          <TablePanel>
            <DataTable
              label="Narrow (morale in the row's details)"
              columns={TABLE_COLUMNS}
              rows={TABLE_FIXTURES.slice(0, 2)}
              rowKey={(row) => row.id}
              rowName={(row) => row.name}
            />
          </TablePanel>
        </div>
        <TablePanel>
          <DataTable
            label="Empty"
            columns={TABLE_COLUMNS}
            rows={[]}
            rowKey={(row) => row.id}
            rowName={(row) => row.name}
            empty="No players match these filters."
          />
        </TablePanel>
        <TablePanel>
          <DataTable
            label="Loading"
            columns={TABLE_COLUMNS}
            rows={[]}
            rowKey={(row) => row.id}
            rowName={(row) => row.name}
            loading
          />
        </TablePanel>
      </div>
    </Section>
  );
}

const PITCH_FIXTURES = [
  { name: "WOUTERS", abbr: "GK", position: "Goalkeeper", ovr: 74, condition: 45, fit: "exact" },
  {
    name: "FRASER",
    abbr: "LM",
    position: "LeftMidfielder",
    ovr: 77,
    condition: 66,
    fit: "adapted",
  },
  { name: "DUPONT", abbr: "RB", position: "RightBack", ovr: 70, condition: 98, fit: "out" },
] as const;

const BADGE_NAMES: Record<string, { label: string; name: string }> = {
  Striker: { label: "ST", name: "Striker" },
  RightWinger: { label: "RW", name: "Right Winger" },
  LeftWinger: { label: "LW", name: "Left Winger" },
  CenterBack: { label: "CB", name: "Center Back" },
};

function PositionBadgeSection() {
  const describe = (position: string) => BADGE_NAMES[position];
  return (
    <Section title="Position badges">
      <div className="flex flex-wrap items-center gap-6">
        <PositionBadge position="Striker" {...BADGE_NAMES.Striker} familiarity="natural" />
        <PositionBadge position="RightWinger" {...BADGE_NAMES.RightWinger} familiarity="adapted" />
        <PositionBadge position="CenterBack" {...BADGE_NAMES.CenterBack} familiarity="out" />
        <PositionBadge position="LeftWinger" {...BADGE_NAMES.LeftWinger} kind="alternate" />
      </div>
      <div className="flex flex-wrap items-center gap-6">
        <PositionBadges
          natural="Striker"
          alternates={["RightWinger", "LeftWinger"]}
          deployed="RightWinger"
          familiarity="adapted"
          describe={describe}
        />
        <PositionBadges
          natural="Striker"
          alternates={["RightWinger"]}
          deployed="CenterBack"
          familiarity="out"
          describe={describe}
        />
      </div>
    </Section>
  );
}

function PitchSection() {
  return (
    <Section title="Pitch tokens">
      <div
        className="flex flex-wrap gap-6 rounded-lg p-6"
        style={{ background: "linear-gradient(var(--color-turf-near), var(--color-turf-far))" }}
      >
        {PITCH_FIXTURES.map((player) => (
          <div key={player.name} className="w-28">
            <PitchToken
              name={player.name}
              positionAbbr={player.abbr}
              position={player.position}
              ovr={player.ovr}
              condition={player.condition}
              fitTone={player.fit}
            />
          </div>
        ))}
      </div>
    </Section>
  );
}

const MARKET_TABS = [
  { id: "players", label: "Players", count: 3915 },
  { id: "list", label: "My list", count: 8 },
  { id: "offers", label: "Offers", count: 5 },
];

// The longest real toolbar: the Transfers centre in German. At 960 wide and extra-large scale its
// tabs don't fit, so they scroll inside the tablist (the page harness measures this).
const LONG_TABS = [
  { id: "list", label: "Meine Transferliste", count: 8 },
  { id: "market", label: "Transfermarkt", count: 3915 },
  { id: "free", label: "Ablösefreie", count: 214 },
  { id: "loans", label: "Leihmarkt", count: 96 },
  { id: "offers", label: "Angebote", count: 5 },
];

function NavigationSection() {
  const [page, setPage] = useState("players");
  const [longPage, setLongPage] = useState("market");
  const [view, setView] = useState("list");
  const [pace, setPace] = useState("normal");
  const [density, setDensity] = useState("comfortable");
  return (
    <Section title="Navigation">
      <PageToolbar
        start={
          <Tabs
            idPrefix="guide-market"
            label="Transfer market"
            tabs={MARKET_TABS}
            selectedId={page}
            onSelect={setPage}
          />
        }
        filters={
          <Select selectSize="sm" aria-label="Position" defaultValue="all">
            <option value="all">All positions</option>
            <option value="def">Defenders</option>
          </Select>
        }
        action={<Button size="sm">Make bid</Button>}
      />
      {MARKET_TABS.map((tab) => (
        <TabPanel key={tab.id} idPrefix="guide-market" tabId={tab.id} hidden={tab.id !== page}>
          <p className="text-body text-fg-muted">The {tab.label.toLowerCase()} view.</p>
        </TabPanel>
      ))}
      <PageToolbar
        start={
          <Tabs
            idPrefix="guide-long"
            label="Transfers, long labels"
            tabs={LONG_TABS}
            selectedId={longPage}
            onSelect={setLongPage}
          />
        }
        filters={
          <Select selectSize="sm" aria-label="Position" defaultValue="all">
            <option value="all">Alle Positionen</option>
          </Select>
        }
        action={<Button size="sm">Transferangebot abgeben</Button>}
      />
      {LONG_TABS.map((tab) => (
        <TabPanel key={tab.id} idPrefix="guide-long" tabId={tab.id} hidden={tab.id !== longPage}>
          <p className="text-body text-fg-muted">{tab.label}</p>
        </TabPanel>
      ))}
      <Row>
        <Tabs
          idPrefix="guide-view"
          label="Attributes view"
          look="segmented"
          tabs={[
            { id: "list", label: "List" },
            { id: "radar", label: "Radar" },
          ]}
          selectedId={view}
          onSelect={setView}
        />
      </Row>
      <TabPanel idPrefix="guide-view" tabId="list" hidden={view !== "list"}>
        <p className="text-body text-fg-muted">The attributes as a list.</p>
      </TabPanel>
      <TabPanel idPrefix="guide-view" tabId="radar" hidden={view !== "radar"}>
        <p className="text-body text-fg-muted">The attributes as a radar.</p>
      </TabPanel>
      <p id="guide-pace" className="text-caption text-fg-muted">
        Speed: a segmented control, strong and stacked
      </p>
      <div className="max-w-sm">
        <SegmentedControl
          labelledBy="guide-pace"
          options={[
            { value: "slow", label: "Slow", icon: <Snail /> },
            { value: "normal", label: "Normal", icon: <Play /> },
            { value: "fast", label: "Fast", icon: <FastForward /> },
            { value: "max", label: "Max", icon: <SkipForward /> },
          ]}
          value={pace}
          onChange={setPace}
          emphasis="strong"
          layout="stacked"
          fill
        />
      </div>
      <p id="guide-density" className="text-caption text-fg-muted">
        Density: a segmented control, quiet and inline
      </p>
      <Row>
        <SegmentedControl
          labelledBy="guide-density"
          options={[
            { value: "comfortable", label: "Comfortable" },
            { value: "compact", label: "Compact" },
          ]}
          value={density}
          onChange={setDensity}
        />
      </Row>
      <PageToolbar action={<Button size="sm">Hire</Button>} />
    </Section>
  );
}

function DialogSection({ open }: { open: boolean }) {
  const [isOpen, setIsOpen] = useState(open);
  return (
    <Section title="Dialog">
      <Row>
        <Button variant="outline" onClick={() => setIsOpen(true)}>
          open the dialog
        </Button>
      </Row>
      {isOpen ? (
        <Dialog
          label="Go to the field?"
          onClose={() => setIsOpen(false)}
          className="w-full max-w-md rounded-lg p-6"
        >
          <div className="flex items-center justify-between">
            <h3 className="font-heading text-heading uppercase tracking-label text-fg">
              Go to the field?
            </h3>
            <DialogCloseButton onClose={() => setIsOpen(false)} />
          </div>
          <p className="mt-2 text-body text-fg-muted">
            Matchday 8 · Brussels FC v Sporting Waregem
          </p>
          <div className="mt-4 flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setIsOpen(false)}>
              cancel
            </Button>
            <Button onClick={() => setIsOpen(false)}>confirm</Button>
          </div>
        </Dialog>
      ) : null}
    </Section>
  );
}

export function StyleGuide({ options }: { options: StyleGuideOptions }) {
  return (
    <main className="min-h-screen bg-page px-6 py-8 text-fg">
      <div className="mx-auto flex max-w-page flex-col gap-10">
        <header className="flex flex-col gap-1">
          <h1 className="font-heading text-title uppercase tracking-label">Style guide</h1>
          <p className="text-caption text-fg-muted">
            {options.theme} · {options.density} · {options.scale}
            {options.highContrast ? " · high contrast" : ""}. Set them with
            ?theme=dark&density=compact&scale=xlarge&contrast=high.
          </p>
        </header>
        <TokensSection />
        <NavigationSection />
        <ControlsSection />
        <DisplaySection />
        <CardsSection />
        <PositionBadgeSection />
        <TablesSection />
        <PitchSection />
        <DialogSection open={options.dialogOpen} />
      </div>
    </main>
  );
}
