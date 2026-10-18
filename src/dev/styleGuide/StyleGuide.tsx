import { Globe, Trophy } from "lucide-react";
import { type ReactNode, useState } from "react";

import {
  Badge,
  Button,
  Card,
  CardBody,
  CardHeader,
  Checkbox,
  CountryFlag,
  DatePicker,
  Dialog,
  DialogCloseButton,
  InjuryBadge,
  JerseyIcon,
  PageToolbar,
  PitchToken,
  PlayerAvatar,
  ProgressBar,
  Select,
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
const TYPE_ROLES = [
  "--text-caption",
  "--text-body",
  "--text-lead",
  "--text-heading",
  "--text-title",
  "--text-display",
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
      <div className="flex flex-col gap-1 rounded-lg border border-line bg-panel p-4">
        {TYPE_ROLES.map((role) => (
          <p key={role} className="text-fg" style={{ fontSize: `var(${role})` }}>
            {role.replace("--text-", "")}: Callum Fraser · Central midfielder · 77
          </p>
        ))}
      </div>
    </Section>
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
      </Row>
      <div className="grid max-w-md gap-2">
        {(["auto", "primary", "accent", "success", "danger", "muted"] as const).map(
          (variant, i) => (
            <ProgressBar key={variant} value={95 - i * 17} variant={variant} showLabel />
          ),
        )}
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

function NavigationSection() {
  const [page, setPage] = useState("players");
  const [view, setView] = useState("list");
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
        <PitchSection />
        <DialogSection open={options.dialogOpen} />
      </div>
    </main>
  );
}
