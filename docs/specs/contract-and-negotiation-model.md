# Feature spec: Contract entity & unified Contract-Negotiation model

- **Status:** Draft for review
- **Target:** implement on a fresh branch off `develop` (this spec is written against
  `develop`, not the abandoned `fix/275-wire-contract-negotiation-into-transfer` branch —
  see [Appendix A](#appendix-a-prior-art) for reusable pieces from that attempt).
- **Supersedes:** issue #275 ("wire contract negotiation into the transfer flow"). Transfer
  personal-terms fall out of this model as one use case rather than a bespoke feature.

---

## 1. Motivation

Contract negotiation is a **core, recurring mechanic**: contract renewals, free-agent
signings, and transfer "personal terms" are all the *same operation* — a club and a player
reaching an agreement on terms (wage + length, later clauses). Today that one concept is
fragmented and denormalized across three unrelated shapes:

| Concept | Where it lives today (`develop`) |
|---|---|
| The **active** contract | loose fields on the player: `player.wage`, `player.contract_end`, and the club side of `player.team_id` |
| The **renewal / free-agent negotiation** | `ContractRenewalState` at `player.morale_core.renewal_state` — a per-player singleton implicitly bound to the *current* club |
| A **prospective (unsigned) contract** | **does not exist** — a transfer that should end in a new contract has nowhere to put the agreed terms |

Consequences of the fragmentation:

- The renewal session **cannot represent a negotiation with any club other than the current
  one**, so a transfer signing can't reuse it.
- There is no object for "a contract we're negotiating but haven't signed," so transfer terms
  have to be bolted onto the transfer offer as parallel fields — a second copy of a structure
  that already exists for renewals.
- Duplicated structures drift. The #275 attempt is the evidence: it shipped a *one-shot*
  transfer-terms flow while renewals were *multi-round*, with missing translations and
  divergent UX, because it copied the concept instead of sharing it.

The data model is the foundation everything else attaches to — release clauses, signing
bonuses, agent fees, loan wage-splits, board approval, contract history. A fragmented
foundation compounds refactor cost with every feature layered on it, and the #275 attempt
already demonstrates the drift: two structures for one concept diverge on the first
feature. Unifying is the smaller total change than layering more features on the
fragmented shape and normalizing later. **Decision: unify, no staging.**

## 2. Goals / non-goals

**Goals**

1. A first-class **`Contract`** entity — a club↔player agreement with an explicit lifecycle
   (`Prospective → Agreed → Active → Expired`). It becomes the single source of truth for
   wage/length, replacing the loose player fields.
2. A single **`ContractNegotiation`** primitive — multi-round, with counters and an
   insult/cooldown, reused by **renewal, free-agent signing, and transfer personal-terms**.
3. A **precondition/guard layer** that decides whether a negotiation may open at all
   (manager "let expire" decision, won't-join-a-rival, dislikes-manager, active cooldown).
4. **Delegation** modelled as a negotiation-general capability (assistant handles it), not a
   renewal-only feature.

**Non-goals**

- **Club-to-club** negotiation (transfer *fee*, loan *terms*) stays as `TransferOffer` /
  `LoanOffer`. This spec is only the club-to-**player** contract. A transfer *composes* the
  two: club-to-club fee → club-to-player contract → sign.
- **Loans' player side.** A loaned player keeps their existing `Contract`; a loan is a
  separate arrangement (wage-split at club level). Loans do not create a player Contract.
- Implementing new economic clauses. The model must *accommodate* them (`ContractTerms` is
  extensible), but adding release clauses/bonuses is future work.

## 3. Locked product decisions (carried from prior design discussion)

- Negotiation is **multi-round** with player counters; it dies only on a genuine breakdown
  (repeated insult → cooldown, or the talks go stale), **not** on a single "no"
  ("breakdown-only kill").
- **Every** transfer that reaches a fee agreement must run a contract negotiation before the
  player moves. Three cases:
  - **AI-to-AI** — negotiation auto-resolves in the same tick (no UI involved).
  - **Managed club, in-window** — the manager negotiates via UI; on agreement the
    transfer executes immediately.
  - **Managed club, out-of-window (deferred)** — the manager negotiates via UI; on
    agreement the prospective contract parks as `Agreed` and executes at
    window-open via §5's atomic replacement.

  The move only completes when both fee and contract are agreed.

## 4. Domain model

Proposed types in the `domain` crate. Names are suggestions; shape is the point.

```rust
/// The negotiable content of an agreement. Extensible (clauses, bonuses later).
pub struct ContractTerms {
    pub weekly_wage: u32,
    pub end_date: String,          // ISO-8601 date, `YYYY-MM-DD` (same shape the
                                   // codebase already uses for other date fields).
                                   // "Length" (remaining or original term) is a
                                   // UI-only derivation. We store end_date because
                                   // expiry is a calendar fact — no ambiguity across
                                   // DST or year boundaries the way a stored length
                                   // + start_date would have.
    // future: release_clause, signing_bonus, appearance_fee, ...
}

pub enum ContractStatus {
    Prospective, // under negotiation, not agreed
    Agreed,      // terms accepted; awaiting execution (e.g. transfer registration)
    Active,      // signed and in force
    Expired,     // ran out / terminated
    Superseded,  // was `Agreed` for the same player when a competing prospective
                 //   contract won the promotion race (§5). Terminal. Unit variant;
                 //   the winner reference lives in the separate `superseded_by`
                 //   field on `Contract` below (not enum-with-data).
}

pub enum ContractSource { Initial, Renewal, FreeAgent, Transfer }

pub struct Contract {
    pub id: String,                     // stable identity — see §7 (Option B).
    pub club_id: String,
    pub player_id: String,
    pub terms: ContractTerms,
    pub status: ContractStatus,
    pub signed_on: Option<String>,      // `None` while `Prospective` and while
                                        // `Agreed` (agreement != signature).
                                        // Set to `today` at the Agreed→Active
                                        // transition (§5 atomic replacement).
                                        // For legacy-migrated Active contracts
                                        // it may stay `None` — see the invariant
                                        // below and §7.
    pub source: ContractSource,
    /// Standing intent, from the manager or the player, for how this
    /// club↔player relationship ends. Contract-scoped; see §6.
    /// Note: **player retirement** is player-global (`player.retirement_intent`),
    /// not a variant here — a free agent can retire without a contract.
    pub exit_intent: Option<ContractExitIntent>,
    /// `Some(winner_id)` when `status == Superseded`, `None` otherwise (unit
    /// variant + separate field, not enum-with-data — the pattern is
    /// `status == Superseded && superseded_by == Some(winner_id)` at every
    /// site, including replay). Enables idempotent-terminal replay: a losing
    /// caller reads this and returns `PromotionRaceLost { winner_id }`
    /// without a second write.
    pub superseded_by: Option<String>,
}

// Invariant (post-migration):
//   `status == Active AND source != Initial ⇒ signed_on.is_some()`.
// Enforced at the Agreed→Active transition, which sets `signed_on = today`.
// Legacy-migrated active contracts (`source = Initial`) are the exception —
// their true signing date is not in the pre-migration save, so `signed_on`
// stays `None` and the UI shows only remaining duration for them. Every
// fresh signing (renewal, free-agent, transfer) writes a real `signed_on`,
// so the exception decays as those contracts get renewed. See §7.

/// A standing decision that ends the club↔player relationship. Contract-scoped:
/// each variant only makes sense in the context of a specific contract with a
/// specific club. Retirement is *not* here — it's player-global (see below).
pub enum ContractExitIntent {
    /// Manager will let the contract lapse; player leaves on a free at expiry.
    /// `reopen_after` gates a manager reversal (carries the legacy
    /// `manager_blocked_until` semantics — until this date, the manager can't
    /// unilaterally re-open renewal talks).
    ManagerRelease {
        set_on: String,
        reason: Option<String>,
        reopen_after: Option<String>,
    },
    /// Player wants out and pushes for a transfer before/at the window.
    PlayerSeekTransfer { set_on: String, cause: DissatisfactionCause },
}

pub enum DissatisfactionCause { LackOfMinutes, LowMorale, Ambition }

/// Player-scoped: retirement survives the contract that carried it (a free
/// agent can announce it). Lives on `Player`, not `Contract`. When set, both
/// renewal and incoming negotiations are hard-blocked; on contract end (or
/// immediately for a free agent) → `player.retired = true`.
pub struct RetirementIntent {
    pub set_on: String,
    pub cause: RetirementCause,
}

pub enum RetirementCause { Age, Injuries }

/// One negotiation session over one prospective contract. Constructed **with**
/// its first offer, so `offered` and `last_activity_on` are always populated
/// from birth — there is no persisted "constructed but no offer yet" state
/// (see `NegotiationStatus` below).
pub struct ContractNegotiation {
    pub status: NegotiationStatus,          // Open | Agreed | Blocked | Stalled
    pub round: u8,                          // starts at 1 with the first offer
    pub offered: ContractTerms,             // last terms the club tabled
    pub suggested: Option<ContractTerms>,   // player's counter
    pub blocked_until: Option<String>,      // insult cooldown (see §5). Distinct
                                            // from `ManagerRelease.reopen_after`
                                            // and from delegate retry timing.
    pub last_activity_on: String,           // ISO date of the last player/club
                                            // exchange (offer, counter, insult).
                                            // Drives the stale rule in §5 and
                                            // the GC question in §10 Q4. Carries
                                            // forward the legacy `last_attempt_date`.
    pub last_delegate_attempt_on: Option<String>, // separate from `last_activity_on`:
                                            // when the assistant last tried a
                                            // delegated round (see §6). Carries
                                            // forward the legacy
                                            // `last_assistant_attempt_date`.
    pub last_outcome: Option<NegotiationOutcome>,
    pub delegated: bool,
}

pub enum NegotiationStatus {
    Open,     // active back-and-forth (initial state — a negotiation is always
              //   constructed with its first offer)
    Agreed,   // terms accepted; ⇒ Contract.status = Agreed
    Blocked,  // insult cooldown; **recoverable** — auto-returns to Open when
              //   `blocked_until` passes (see §5)
    Stalled,  // terminal: no activity for N days (§5, N = 14). Prospective
              //   contract is GC'd; existing renewal ends without a signature.
              //   Transition also sets `last_outcome = Some(TimedOut)` (§5),
              //   overwriting any prior `Countered` / `Insulted`.
}

pub enum NegotiationOutcome {
    Accepted,   // terms met (⇒ status Agreed)
    Countered,  // player suggested different terms
    Insulted,   // offer far below floor (⇒ status Blocked, blocked_until set)
    TimedOut,   // reached staleness (⇒ status Stalled)
}
```

### Cardinality (the key insight)

A negotiation is **a singleton per contract**, and a *contract* is the club↔player relation —
so the "where does it live" question that blocked reuse disappears:

- A player has **exactly one `Active` contract**, owned by `contract.club_id`. The
  contract's club is the **parent** — not `player.team_id` — so a loaned player keeps
  their active contract with the parent while `team_id` points at the borrower. `Contract`
  **replaces** `player.wage` / `player.contract_end` as the source of truth (and the
  contract owner replaces the "loose `team_id` is the employer" assumption).
- A player may have **N `Prospective` contracts** at once — competing buyers, and/or a
  renewal draft with the current parent club. Each prospective contract owns **one**
  `ContractNegotiation`.
- **Renewal** = a prospective contract whose `club_id` matches the *active contract's
  owner*; on sign it replaces the active one. (During a loan, that's the parent, not the
  borrowing club.)
- **Free-agent signing** = a prospective contract while the player has no active contract.
- **Transfer** = after the club-to-club fee is agreed, a prospective contract with the
  *buying* club + its negotiation; on agreement the transfer executes and the prospective
  contract becomes `Active` (old one `Expired`).

## 5. Lifecycle & state machine

```text
Contract.status:
  Prospective ──negotiation Agreed──▶ Agreed ──executed / registered──▶ Active ──ends──▶ Expired
        │                                  │
        │                                  ├─ competing prospective won the promotion
        │                                  │  race (§ Atomic replacement) ──▶ Superseded
        └─ negotiation → Stalled ─────────▶ (contract discarded)

ContractNegotiation.status (per prospective contract):
  (constructed with first offer) ──▶ Open (round = 1)
  Open ──counter──▶ Open (round++)
  Open ──insult──▶ Blocked (cooldown; **recoverable**, not terminal)
  Blocked ──today ≥ blocked_until──▶ Open (auto-return; round stays)
  Open ──terms met──▶ Agreed (⇒ Contract.status = Agreed; terminal-success)
  Open|Blocked ──today − last_activity_on ≥ STALE_N──▶ Stalled + last_outcome
                                                    = Some(TimedOut)
                                                    (terminal-failure;
                                                     contract discarded)
```

**Stale threshold.** `STALE_N = 14` calendar days, matching the existing renewal
cadence in `develop`. One named constant, used consistently by the state machine
above, prospective-contract GC (§7), and the §9 tests. A future tuning pass may
make it configurable per source (renewal vs transfer), but v1 uses one value.

**GC scope.** The `STALE_N` rule applies **only to prospective contracts whose
negotiation reached `Stalled`** — those get discarded together with their
negotiation, per the diagram above. Parked `Agreed` contracts (an out-of-window
transfer waiting on the window opening) are **not** GC'd by `STALE_N` — they
have their own expiry rule tied to the transfer window's close date, not to
`last_activity_on`. `Superseded` (from the promotion race) is terminal and not
GC'd either — it's kept as an audit tombstone with `superseded_by`.

**Precedence when Blocked and staleness collide** (`blocked_until` in the past AND
`last_activity_on` also stale): **staleness wins.** A negotiation that sat idle past
`STALE_N` days is dead even if its cooldown happens to have expired in the meantime
— the party who insulted never came back.

### Atomic active-contract replacement

The "exactly one `Active` contract per player" invariant (§4) means every place the
active contract changes must be a **single atomic transition**, not a sequence of
independent status writes. Applies to renewal, transfer, and free-agent signing.

The transition **moves** the same `Contract` record (identified by `Contract.id`)
out of the prospective collection and into the active slot — it never clones. Under
Option A that means: pop the entry from `player.contract_negotiations` whose
`contract.id` matches, set its `status = Active` and `signed_on = today`, and
assign it to `player.contract`. Under Option B it's a single UPDATE of the
row's `status` (the row's `id` is already stable).

```text
old.status: Active → Expired                  ┐
new: taken from contract_negotiations,        │ one transaction
     status Agreed → Active, signed_on set,   ├ (save-write barrier;
     assigned to player.contract              │  serialized per player,
player.retirement_intent (only if applicable) ┘  see below)
```

**Idempotent-retry semantics.** If the transition committed but the caller
crashed before observing success, replaying the same call is a no-op. The check
is: does `player.contract.id == new.id`? If yes, the transition already ran —
succeed. This works even though `new` is no longer in `player.contract_negotiations`
(the move already happened) because `Contract.id` is stable across the move.

**Concurrent promotions.** Two `Agreed` prospective contracts for the same player
(e.g. a renewal and a transfer both ready to sign in the same tick) race for the
one `Active` slot. Resolution: promotion is **serialized per player** at the
save-write barrier — a per-player mutex under Option A, a compare-and-swap on
`player.contract_id` or a partial-unique-index conflict under Option B. First
writer wins.

The loser is closed **inside the same transaction** as the winner's promotion,
not deferred to caller cleanup. Concretely, when the barrier is held, the
transaction:

1. promotes the winner (`Agreed → Active`, move into `player.contract`);
2. transitions every *other* `Agreed` prospective contract for the same player
   from `Agreed` to `Superseded` (a new terminal `ContractStatus` variant),
   setting `superseded_by = Some(winner_id)`; and
3. moves those loser contracts out of `player.contract_negotiations` (whose
   contract is `Prospective | Agreed`) into a **`player.contract_history:
   Vec<Contract>`** collection under Option A — `Superseded`, `Expired`, and
   any other terminal status that we want to keep as an audit tombstone lives
   there. Under Option B the row simply flips `status = Superseded` and its
   `contract_negotiations` row is deleted (see §7 Option B lifecycle rules
   below).

`Superseded` is terminal (like `Expired`) and never GC'd by the stale rule.
This makes the outcome durable in one write: the loser cannot linger as a
live prospective agreement even if the caller crashes before observing the
result. On replay, a losing caller looks up its `Contract.id` — under Option
A it searches both `player.contract_negotiations` and `player.contract_history`
— and sees `status == Superseded && superseded_by == Some(winner_id)`, then
returns `PromotionRaceLost { winner_id }` from a pure read (no second write,
idempotent-terminal).

**Fee-rollback crash safety.** The caller's follow-up — "cancel the fee
agreement because we lost the personal-terms race" — is not part of the
promotion transaction (the fee is club-to-club state, owned by a different
subsystem). To keep it crash-safe, fee rollback is an **idempotent operation
keyed by `(loser_contract.id, winner_id)`**: it flips the fee agreement to
`Cancelled` with `cancel_reason = LostPersonalTermsRace { winner_id }` if
still `Agreed`, and is a no-op if already `Cancelled` with the same key.
Replay of a losing caller therefore always converges the fee to `Cancelled`
regardless of when it crashed — the fee-rollback call and the pure
`PromotionRaceLost` read together are a two-step idempotent recovery.

Renewal treats losing the race as "player already signed elsewhere" and
closes the renewal draft; the barrier already did the closing, so there's
nothing to roll back — this is just UI.

**Free-agent signing** is a degenerate case (no old contract to expire): the
`Prospective` → `Agreed` → `Active` move happens under the same serialized
barrier, with `player.contract = Some(new)`.

Mapping the transfer flow onto this (replaces the #275 `PersonalTermsPending` /
`PersonalTermsFailed` offer states):

| Transfer step | Contract | Negotiation |
|---|---|---|
| Fee agreed, guard permits | Prospective contract created for buyer | `Open`, round 1 |
| Fee agreed, guard hard-blocks | the fee agreement is already persisted (it's the earlier club-to-club negotiation). In one transaction: flip the fee agreement to `Cancelled`, write the typed rejection (§6), and **do not create a prospective contract**. Atomic + idempotent on retry — replay observes the fee agreement already `Cancelled` and rejection already written, returns success. | (never constructed) |
| User/AI negotiating wage | Prospective | `Open`/`Blocked` |
| Terms agreed, in-window | Agreed → executed → **Active** (atomic; see above) | `Agreed` |
| Terms agreed, out-of-window | **Agreed** (parked for registration; execution deferred to the window's atomic transition) | `Agreed` |
| Talks stale N days | contract discarded | `Stalled` |

## 6. Precondition / invariant guard

Before a negotiation may **open** (or auto-open via delegation), a guard evaluates gates.
This consolidates checks that today are scattered inside the renewal session logic.

Gates come in two flavours — **hard blocks** (a negotiation may not open) and **dispositions**
(it may open but its odds/dynamics shift). They mix **stored** flags and **computed**
predicates.

Two intent-shaped gates. `Contract.exit_intent` (contract-scoped) and
`Player.retirement_intent` (player-scoped) each shape or block negotiations:

| Intent | Where stored | Effect on renewal | Effect on incoming transfer/contract |
|---|---|---|---|
| `ManagerRelease` | `Contract.exit_intent` | **hard block** — manager chose to release; `reopen_after` gates a reversal | allowed — the manager has already released the player |
| `PlayerSeekTransfer` | `Contract.exit_intent` | **disposition** — likely refuses / demands a premium | **disposition** — lower resistance, may hand in a request |
| `RetirementIntent` | `Player.retirement_intent` | **hard block** | **hard block** — the player has decided to stop; on contract end (or immediately for a free agent) → `player.retired = true` |

`exit_intent` is genuinely *contract/relationship* state — it doesn't survive the contract
that carried it — which is why it lives on `Contract`, not on `ContractNegotiation` or
`Player`. Retirement is genuinely *player-global* — a veteran without a club can still
announce it — which is why it lives on `Player`, not on `Contract`.

**Which record does the guard read?** For **renewal** and any *outgoing* action by the
current club, `exit_intent` comes from that club's own active contract (where it was
set); `retirement_intent` comes from the player. For an **incoming** transfer / free-agent
negotiation, `exit_intent` comes from the player's **active** contract with the current
club (the newly created prospective contract for the buying club starts with
`exit_intent = None` and inherits nothing); `retirement_intent` still comes from the
player. A free agent has no active contract to read, so `exit_intent` is absent by
construction — only `retirement_intent` applies, which matches the table above:
`ManagerRelease` (contract-scoped) can't survive without a contract, `RetirementIntent`
does.

Other gates:

- **Active cooldown** — `blocked_until` in the future (from a prior insult). Transient block.
- **Won't join a rival club** — computed from club relationships (disposition/hard block; future).
- **Dislikes the manager / poor relationship** — computed from morale/relationship (today
  partially exists as `should_manual_renewal_fail_on_relationship`).

The guard returns "may negotiate", "blocked (typed reason)", or "allowed with modifiers" — the
typed reason drives UI messaging + i18n. "Singleton validates invariants first": construct /
allow a negotiation only when the guard permits, and feed any disposition modifiers into the
negotiation.

## 7. Persistence

The game is an in-memory object graph serialized to SQLite (players are rows with JSON
blobs). Two options for the **active** contract:

- **(A) Embedded on the player** — `player.contract: Option<Contract>` replaces the loose
  `wage`/`contract_end` fields (club side still mirrored by `team_id`). No new table; keeps
  the object-graph style. Prospective contracts + negotiations attach to their initiating
  context (transfer offer, renewal intent, free-agent intent).
- **(B) Normalized `contracts` table** — active + prospective contracts as rows
  primary-keyed by `Contract.id` (stable across status changes and safe for
  `TransferOffer` foreign keys). Separate constraints enforce the semantics: a partial
  unique index on `(player_id) WHERE status = 'Active'` for the one-active invariant
  and a non-unique `(player_id, club_id, status)` lookup index for the negotiation UI.
  `ContractNegotiation` state lives in a separate `contract_negotiations` table
  primary-keyed by `Contract.id` (one-to-one with the prospective contract, foreign
  key with `ON DELETE CASCADE`); each row holds `status`, `round`, `offered`
  (serialized `ContractTerms`), `suggested`, `blocked_until`, `last_activity_on`,
  `last_delegate_attempt_on`, `last_outcome`, and `delegated`. A JSON blob for the
  full `ContractNegotiation` in that table is acceptable if we want to keep the
  schema shallow; either way it must be restorable and updatable from Option B's
  documented storage.

  **Negotiation-row lifecycle on status change (Option B).** The
  `contract_negotiations` row is meaningful only while its contract is
  `Prospective` or `Agreed`. Terminal transitions delete it atomically with
  the `Contract.status` update, in one transaction:

  | Contract transition | `contract_negotiations` row |
  |---|---|
  | `Agreed → Active` (promotion) | `DELETE` |
  | `Agreed → Superseded` (lost race) | `DELETE` |
  | `Prospective → discarded` (Stalled GC) | `DELETE` (cascades via the row's `ON DELETE CASCADE` on the contract row, when the contract row itself is deleted; if the contract row is kept as an audit tombstone, delete the negotiation row explicitly) |
  | `Active → Expired` | n/a (already deleted at the previous transition) |

  Idempotent replay: if the row is already absent, the delete is a no-op —
  repeating the promotion (or the loss cleanup) converges to the same
  `contracts` row state and no `contract_negotiations` row, no
  duplicate-write conflict.

**Recommendation: (A) embedded**, unless a reviewer wants full normalization. (A) delivers
the unification (one `Contract`/`ContractNegotiation` type, one negotiation engine, one UI)
at far lower churn, and matches how the rest of the game state is stored. Revisit (B) only if
contract *history* or cross-club contract queries become first-class needs.

Prospective-contract storage (under option A): a single
`player.contract_negotiations: Vec<ProspectiveContract>` where
`ProspectiveContract { contract: Contract (status=Prospective|Agreed), negotiation:
ContractNegotiation }`. This is club-keyed (`contract.club_id`), so it naturally supports
competing buyers and a renewal draft simultaneously, and removes the need to store terms on
the `TransferOffer` at all — the offer references the prospective contract.

Terminal-status audit storage (under option A): `player.contract_history:
Vec<Contract>` holds contracts whose status is `Expired` or `Superseded` — the
audit tombstones. On the atomic transitions in §5, the contract record is
moved out of `player.contract` (Expired) or `player.contract_negotiations`
(Superseded) and appended to `player.contract_history`, in the same
transaction. Replay lookups for `PromotionRaceLost` search both the
prospective collection and the history so a losing caller can find its
`Superseded` record regardless of when it crashed.

**Migration:** ~25 Rust files read `player.wage`; a similar set read `contract_end`, plus the
frontend. All become `player.contract.terms.weekly_wage` / `…end_date` (or an accessor). This
is a mechanical but wide change — the main cost of the refactor.

Save migration covers three shapes:

- **Active contract** — synthesize a `Contract` from `wage` + `contract_end`, with
  `club_id` resolved to the contract owner (**parent club during a loan**, not
  `player.team_id`, so renewal/termination permissions stay with the parent). Set
  `source = Initial` on the synthesized contract; this both marks it as
  migration-origin and admits the `signed_on` exception spelled out in §4 — the
  pre-migration save has no signing date, so `signed_on` migrates to `None` and
  the "original term length" UI shows only remaining duration. Every fresh
  signing (renewal, free-agent, transfer) creates a contract with `source ≠
  Initial` and a real `signed_on`, so the exception decays as `Initial` contracts
  turn over. No fabricated dates.
- **Renewal session** — `player.morale_core.renewal_state` (`ContractRenewalState`) fields
  map to distinct targets — they carry different semantics and must not be lumped
  together:

  | Legacy field | New home | Notes |
  |---|---|---|
  | `status = Idle` | **no renewal draft created** | `Idle` means "no offer tabled" — the new model has no such state (§4). Drop the legacy session; the player just has no in-flight renewal, which is what `Idle` meant. |
  | `status = Open` \| `Blocked` | new prospective `Contract` (`status = Prospective`, `source = Renewal`) + `ContractNegotiation` with a synthesized first offer — see next rows | preserves the invariant `NegotiationStatus == Open` ⇒ `Contract.status == Prospective` |
  | `status = Agreed` | new prospective `Contract` **with `status = Agreed`** + `ContractNegotiation` (`status = Agreed`) — see next rows | preserves `NegotiationStatus::Agreed ⇒ Contract.status = Agreed` (§4). The next tick's atomic-replacement step promotes it to `Active` normally, so no in-flight-renewal is dropped |
  | `round`, `last_outcome` | direct copy | |
  | `blocked_until` (insult cooldown from the shared evaluator) | `ContractNegotiation.blocked_until` | direct copy — same semantics |
  | `last_attempt_date` (present) | `ContractNegotiation.last_activity_on` | already the same concept (Appendix A) |
  | `last_attempt_date` (absent) | `ContractNegotiation.last_activity_on = migration_date` | required field; absent-legacy value defaults to the migration date so the stale clock starts at migration, not at some fabricated past date |
  | `last_assistant_attempt_date` | `ContractNegotiation.last_delegate_attempt_on` | delegate-retry timing, **separate** from the negotiation's last activity — do not overwrite `last_activity_on` |
  | *(no legacy `offered`)* | `ContractNegotiation.offered = player.current_terms` | synthesize from the player's current `wage` + `contract_end` — that's the club's known baseline position. Better than a placeholder, and consistent with "constructed with first offer" (§4). |
  | *(no legacy `suggested`)* | `ContractNegotiation.suggested = None` | player hadn't tabled a counter — legitimate absence |
  | *(no legacy `delegated`)* | `ContractNegotiation.delegated = false` | assistant hand-off wasn't recorded; assume manager-driven, which is the safer default (delegation reactivates on next manager click) |
  | `exit_intent = LetExpire` | **active** contract's `exit_intent = ManagerRelease` | see below |
  | `manager_blocked_until` | active contract's `ManagerRelease.reopen_after` | manager-reversal gate; only carried when `LetExpire` is set (that's what it gates) |

  Putting migrated `LetExpire → ManagerRelease` on the **active** contract (not the
  prospective renewal draft) is deliberate: §6 says the renewal guard reads
  `exit_intent` from the active contract. Migrating it to the prospective draft would
  hide it from the guard and silently flip existing saves back to "renewal open."
  For the mutually exclusive case where a save has both `LetExpire` and an in-flight
  session, the migration keeps the intent on the active contract *and* discards the
  prospective draft — `LetExpire` semantically means the renewal has already been
  declined, so the draft is stale by definition.
- **Retirement intent** — legacy saves don't currently persist a player-scoped
  retirement intent, so `player.retirement_intent` migrates to `None`. Engine-driven
  retirement (age/injury thresholds) continues to set it going forward (see §10 Q7).

An alternative path is to drop legacy-save support outright; if we choose that, the
migration commit must say so explicitly rather than leaving the in-flight session state
unspecified.

## 8. Impact / call sites to touch

- `domain/player.rs` — new `Contract` (with `ContractStatus::Superseded` +
  `Contract.superseded_by: Option<String>` — unit variant + separate field,
  not enum-with-data), `ContractTerms`, `ContractNegotiation`,
  `NegotiationStatus` (rename of `RenewalSessionStatus`), `NegotiationOutcome`,
  `ContractExitIntent` (contract-scoped: `ManagerRelease`,
  `PlayerSeekTransfer`), `RetirementIntent` (player-scoped, on
  `Player.retirement_intent`). New collections on `Player` (Option A):
  `player.contract_negotiations` for `Prospective | Agreed`, plus
  `player.contract_history` for `Expired | Superseded` tombstones. Replace
  `player.wage`/`contract_end` with optional getters (returning `Option<u32>`
  / `Option<String>`); replace `morale_core.renewal_state`.
- `ofm_core/contracts.rs` — renewal + free-agent rebuilt on the shared negotiation engine +
  guard.
- `ofm_core/transfers.rs` — fee agreement creates a prospective contract; personal-terms use
  the shared negotiation; execution promotes Prospective→Active.
- `ofm_core/delegated_renewals.rs` — becomes "delegate contract negotiations" (guard-aware).
- `ofm_core/squad_safety.rs`, contract-expiry, `finances.rs` (wage bill), player events
  (renewal nags) — read the active contract via the new accessor; respect the guard.
- Frontend: player profile (renewal), transfers (fee → contract negotiation), a single
  `useContractNegotiation` hook + one modal replacing `useFreeAgentContractFlow` and the
  (planned) transfer-terms hook/modal.
- i18n: one set of negotiation strings + guard-reason error keys, across all locales.

## 9. Testing strategy

- **Unit** — the shared negotiation evaluator (accept / counter / insult→cooldown /
  stale) and the guard (each gate) — one test suite, exercised by all three flows.
- **Contract lifecycle** — Prospective→Agreed→Active→Expired transitions; renewal
  replaces active atomically (idempotent retry test — replaying the same commit is
  a no-op, verified by `player.contract.id == promoted.id`); the promotion **moves**
  the same `Contract.id` from `player.contract_negotiations` to `player.contract`
  (no clone); transfer promotes prospective; free-agent creates first contract.
- **Concurrent-promotion race** — two `Agreed` prospective contracts for the same
  player promoted in one tick, only one wins; loser is transitioned to
  `status = Superseded` with `superseded_by = Some(winner_id)` **inside the
  winner's transaction** (unit variant + separate field; never enum-with-data).
  Post-transaction assertions:
  - Option A: winner is in `player.contract`, loser is in
    `player.contract_history` (not `contract_negotiations`);
  - Option B: winner row has `status = Active`, loser row has
    `status = Superseded`, both `contract_negotiations` rows are gone.
  **Loser crash+replay** — kill the losing caller between the barrier and its
  fee-rollback call, then re-drive it: (a) it must read `Superseded +
  superseded_by = Some(winner_id)` and return `PromotionRaceLost { winner_id }`
  from a pure read, and (b) the idempotent fee-rollback op (keyed by
  `(loser_contract.id, winner_id)`) must converge the fee to `Cancelled` no
  matter how many times it runs — verify the fee state is `Cancelled` with
  `cancel_reason = LostPersonalTermsRace { winner_id }` after both first
  execution and a repeated replay.
- **Negotiation state machine** — `Blocked → Open` auto-return when `blocked_until`
  passes (must not skip a round), `Open|Blocked → Stalled` at `STALE_N` days,
  precedence when both trigger on the same tick (staleness wins). Stale transition
  sets `last_outcome = Some(TimedOut)`, overwriting any prior `Countered`/`Insulted`.
  Constructor test: a fresh negotiation is `Open`, round 1, with a populated first
  offer and `last_activity_on = today` (no `Idle` gap where those fields would be
  absent).
- **Flow integration** — renewal, free-agent, transfer (in-window sign, out-of-window
  defer→register), each reaching a definite end. Transfer guard-block path: a fee-agreed
  transfer against a `RetirementIntent` player flips the fee agreement to `Cancelled`
  and writes the typed rejection in one transaction, with **no prospective contract
  persisted**; replay after crash observes the same state.
- **Absence cases** — `player.wage()` / `player.contract_end()` return `None` for
  free agents; `finances.rs` (wage bill) and the FinancesTab consumers handle the
  optional cleanly; a loaned player's active contract resolves to the parent club as
  owner and passes renewal-permission checks against the parent, not the borrower.
- **Legacy-save migration** — round-trip a save from `develop` through the migration:
  `wage`/`contract_end`/`team_id` → `Contract` with `source = Initial` and
  `signed_on = None`; `renewal_state` fields land in the right targets per the
  mapping table (§7); `LetExpire` on the *active* contract hard-blocks renewal via
  the guard; `manager_blocked_until` becomes `ManagerRelease.reopen_after`;
  `last_assistant_attempt_date` becomes `last_delegate_attempt_on` (not
  `last_activity_on`). One test per legacy `RenewalSessionStatus`:
  - `Idle` → no renewal draft created;
  - `Open` / `Blocked` → prospective `Contract` with `status = Prospective` +
    `ContractNegotiation` with the corresponding status;
  - `Agreed` → prospective `Contract` with **`status = Agreed`** +
    `ContractNegotiation` with `status = Agreed` (preserves the
    `NegotiationStatus::Agreed ⇒ Contract.status = Agreed` invariant; a
    subsequent tick promotes it to `Active` via the normal atomic path);
  - all live statuses share the synthesized field defaults —
    `offered = player.current_terms`, `suggested = None`, `delegated = false`,
    and `last_activity_on = migration_date` when the legacy value is absent.

  These tests run **before** the loose fields and `renewal_state` are deleted,
  not after.
- **Regression parity** — renewal/free-agent behaviour unchanged vs `develop` (same
  accept/counter thresholds).

## 10. Open questions for the reviewer

1. **Persistence:** embedded (option A) or normalized `contracts` table (option B)?
2. ~~**Terms canonical form:** `length_years` vs `end_date`?~~ **RESOLVED:** store
   `end_date`. Both are fixed at signing — a stored term length would not "drift" as
   the clock advances (only *remaining* duration changes, and that's derived either
   way). `end_date` wins because it is the direct calendar fact contracts turn on
   (window closes, expiry triggers), with no DST/year-boundary ambiguity. "Length" is
   a UI-only derivation (`end_date − start_date` for original term, `end_date − today`
   for remaining).
3. ~~**Active contract on the player:** hard-replace `wage`/`contract_end` or keep accessors?~~
   **RESOLVED:** `Contract` is the only *stored* field; `player.wage()` and
   `player.contract_end()` become **getter methods** returning `Option<u32>` and
   `Option<String>` respectively — they read through `player.contract` and return
   `None` for free agents (`player.contract.is_none()`). Rust can't hold a real
   pointer into a co-owned field (that's a self-referential struct), so a method is
   the idiomatic "pointer to contract.wage." Persistence round-trips cleanly (only the
   `Contract` is serialized; the getters are never written, so no second copy / no
   drift). Consequences:
   - **Backend consumers** — `finances.rs` (wage bill), squad-safety, contract-expiry
     events all switch to the optional; a free agent contributes `0` to the wage bill
     and never triggers an expiry event.
   - **Frontend** — the player JSON loses top-level `wage`/`contract_end`; the TS side
     reads `player.contract?.terms.*` instead. `FinancesTab` computes wage bill and
     expiry risk from the optional (missing = 0 / not-at-risk), replacing the current
     required-value calculations.
4. ~~**Prospective-contract GC:** reuse the same `STALE_N` (= 14) `last_activity_on`
   stale rule (§5) to discard abandoned prospective contracts alongside their
   negotiations?~~ **RESOLVED:** yes for `Stalled` prospective contracts (they
   discard together with the negotiation). Parked `Agreed` contracts have their
   own expiry rule tied to the transfer window's close date, not to
   `last_activity_on`, and are **excluded** from `STALE_N` GC. `Superseded`
   contracts are terminal audit tombstones and are not GC'd either. See §5 "GC
   scope."
5. **Guard reasons:** which gates ship in v1 (definitely `exit_intent` + cooldown; relationship
   and rival-club now or later)?
6. **Loan wage-split:** confirmed out of scope for the Contract entity (stays club-level)?
7. **Player-stated exit intents — how are they triggered?** Engine-driven from thresholds
   (age + form/minutes → retire; sustained low morale / bench time → seek transfer; injury
   history → retire), surfaced to the manager as a player event? Or manager-visible only via
   the negotiation guard? These likely tie into the existing player-events / morale system.
8. ~~**Where does retirement intent belong?**~~ **RESOLVED:** hoisted to
   `Player.retirement_intent: Option<RetirementIntent>`. Retirement genuinely survives
   the contract that carried it (a free agent can announce it), while `ManagerRelease`
   and `PlayerSeekTransfer` are contract/relationship-scoped and stay on `Contract`.
   The guard reads it as a player-level gate independently of any contract lookup.

---

## Appendix A: prior art

The abandoned `fix/275-…` branch implemented much of the *negotiation logic* against the old
fragmented model. Reusable when building this properly:

- **`evaluate_contract_terms(...)`** — the shared, multi-round evaluator
  (accept / counter / insult) with a wage baseline parameter. Lift this onto `ContractTerms`.
- **`ContractTalksStatus`** (rename of `RenewalSessionStatus`) → this spec's
  `NegotiationStatus`. Prior art carried an `Idle` variant meaning "constructed
  but no offer yet"; this spec drops it (§4) because negotiations are constructed
  with their first offer. Legacy `Idle` sessions are handled by the migration
  rule in §7.
- Multi-round + `blocked_until` cooldown mechanics and the "breakdown-only kill" rule.
- Frontend: `PersonalTermsModal` + `useTransferPersonalTermsFlow` show the intended UX
  (round/counter/patience display, cooldown-disables-submit) — fold into the single
  `useContractNegotiation` hook + modal.
- i18n error keys added there (`be.error.transfers.personalTerms*`, `wageBudgetExceeded`,
  `notBuyingTeam`) — reuse/rename under a contract-negotiation namespace.

## Appendix B: why not keep the two structures

Considered and rejected: keep `ContractRenewalState` (player) and transfer-terms (offer)
separate, sharing only the evaluator. Rejected because (a) it leaves two state shapes, two
hooks, and two modals for one concept — the exact surface that drifted in #275 — and (b) the
"they live in different places" objection dissolves once `Contract` is first-class: both are
negotiations over a contract, so there is one natural home. The only genuinely
context-specific field, `exit_intent`, is a property of the *contract/relationship* (a gate),
not of the negotiation session, and is modelled as such in §6.
