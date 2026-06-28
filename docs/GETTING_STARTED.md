# Getting Started

Welcome to **OpenFoot Manager** — an open-source football management simulation where you take charge of a club, build your squad, set tactics, and lead your team through a full league season.

This guide walks you through the basics of gameplay from creating your manager to lifting the trophy.

---

## Creating a New Game

### 1. Create Your Manager

From the main menu, click **New Game**. You'll be asked to fill in your manager profile:

- **First Name** and **Last Name**
- **Date of Birth** (your manager must be at least 30 years old)
- **Nationality** (choose from the full list of nationalities)

### 2. Choose a World

After filling in your manager details, you'll pick the world database:

- **Random World** — A procedurally generated league with teams, players, and staff. Every game is different.
- **Custom Database** — If you have a custom world database JSON file, you can import it here.

### 3. Choose Your Club

You'll see all the teams in the league displayed as cards. Each card shows:

- **Reputation** — World Class, Strong, Average, or Developing
- **Squad Size** — Number of players
- **Finances** — Current balance
- **Average OVR** — Overall squad quality (higher is better)
- **Stadium** — Name and capacity

Click a team to select it, then confirm. You're now the manager!

---

## The Dashboard

The Dashboard is your command centre. It's organized into a **sidebar** with navigation tabs, a **header** with key information and the Continue button, and the **main content area**.

### Sidebar Sections

**General:**
- **Home** — Overview of your club: next fixture, league position, recent messages, and a quick squad summary
- **Inbox** — Messages from the board, staff, scouts, and the league office. Read match previews, result reports, and fitness warnings here
- **Manager** — Your career stats: matches managed, win rate, trophies

**Club:**
- **Squad** — Your full player roster with attributes, condition, value, and overall rating
- **Tactics** — Set your formation and play style, view the pitch layout
- **Training** — Choose training focus, intensity, and weekly schedule
- **Staff** — Manage your coaching and medical staff (hire and release)
- **Finances** — Club balance, wage budget, transfer budget, and payroll breakdown
- **Transfers** — Transfer market, loan market, and your transfer-listed players

**World:**
- **Players** — Browse every player in the league with filters and sorting
- **Teams** — View all teams in the league with key stats
- **Tournaments** — League standings and full fixture results
- **Schedule** — Your calendar of upcoming and past fixtures
- **News** — Read match reports, league roundups, and standings updates

---

## Day-to-Day Gameplay

Openfoot Manager advances **one day at a time**. Each day either has a match or is a training day.

### Advancing Time

The **Continue** button in the top-right of the Dashboard is your main control. Click the arrow to see your options:

- **Go to the Field** — Play the match live with full tactical control
- **Watch as Spectator** — Watch the match unfold without being able to make changes
- **Delegate to Assistant** — Simulate the match instantly (results calculated in the background)

On non-match days, clicking Continue advances to the next day, processing training for all teams.

### Skip to Match Day

If you don't want to advance day-by-day, use **Skip to Match Day** to fast-forward through training days straight to the next fixture. Training will still be processed for each skipped day.

---

## Setting Up Your Tactics

### Formation

Go to the **Tactics** tab and pick a formation. Available formations include:

- **4-4-2** — Classic balanced shape
- **4-3-3** — Wide attacking with wingers
- **3-5-2** — Three at the back with wing-backs
- **4-2-3-1** — Defensive midfield shield with an attacking midfielder
- **4-5-1** — Compact and defensive
- **5-3-2** — Solid defence with two strikers

The pitch visualization shows how your players are positioned in the chosen formation.

### Play Style

Your play style affects how the team behaves during matches:

| Style | Effect |
|-------|--------|
| **Balanced** | No modifiers — solid all-round approach |
| **Attacking** | Better in attack, weaker in defence |
| **Defensive** | Stronger defence, fewer goals scored |
| **Possession** | Better passing, but slightly less direct |
| **Counter** | Strong on the break, weaker when holding the ball |
| **Pressing** | Win the ball higher up, but use more energy |

Choose a style that suits your squad's strengths. A team full of fast forwards may thrive on Counter, while a midfield-heavy squad might prefer Possession.

### Phase Blueprint

Below the play style is the **Phase Blueprint** — finer dials that shape *how* your team plays in three moments: when you have the ball, when you don't, and in the seconds right after possession changes hands. The same blueprint is available on the pre-match screen so you can adjust it for a specific opponent.

Every dial has a sensible middle setting; leaving the blueprint untouched plays a balanced game. Pushing a dial trades one quality for another, so pick the few that fit your players and game plan rather than maxing everything. Effects are deliberately subtle on their own but add up across a match and a season.

**With the ball**

| Dial | What it does |
|------|--------------|
| **Build-up** | *Short* keeps possession safer at the back; *Long* is more direct and riskier. |
| **Width** | *Wide* works the flanks and crosses more; *Narrow* plays through the middle. |
| **Tempo** | *Patient* circulates and holds the ball; *Direct* moves it forward faster and shoots more. |

**Without the ball**

| Dial | What it does |
|------|--------------|
| **Defensive line** | A *high* line wins the ball back higher up but concedes better chances in behind; a *low* line is more cautious. |
| **Pressing** | *Aggressive* wins the ball back more often but tires your players faster; *Passive* conserves energy. |
| **Defensive shape** | *Compact* makes you harder to create chances against; *Stretched* covers more ground but concedes more. |
| **Marking** | *Man-to-man* sticks tight (and fouls more); *zonal* holds position. |

**Transitions**

| Dial | What it does |
|------|--------------|
| **Counter-press** | How long you keep chasing the ball straight after losing it: *None* (don't), *Short*, or *Long* — the longer you press, the more often you win possession back. |
| **Break speed** | How quickly you attack after winning the ball. *Fast* turns turnovers into quick chances; *Medium* (default) and *Slow* play it safe and don't trigger fast counters. |

As a starting point: a possession side might pick Short build-up, Patient tempo and a Long counter-press; a counter-attacking side might pick Direct tempo, a Compact shape and Fast breaks.

---

## Training

Training happens automatically on non-match days, but you control three settings:

### Training Focus

Pick which attributes your players develop:

| Focus | What Improves |
|-------|--------------|
| **Physical** | Pace, stamina, strength, agility |
| **Technical** | Passing, shooting, dribbling |
| **Tactical** | Positioning, vision, decisions, composure |
| **Defending** | Tackling, defending (+ some strength and positioning) |
| **Attacking** | Shooting, dribbling (+ some pace) |
| **Recovery** | No attribute gains, but maximum condition recovery |

### Training Intensity

| Intensity | Growth Rate | Condition Cost |
|-----------|------------|----------------|
| **Low** | Slow | Minimal fatigue |
| **Medium** | Normal | Moderate fatigue |
| **High** | Fast | Heavy fatigue |

### Weekly Schedule

| Schedule | Training Days | Rest Days |
|----------|-------------|-----------|
| **Intense** | 6 days (Mon–Sat) | 1 (Sun) |
| **Balanced** | 4 days (Mon, Tue, Thu, Fri) | 3 (Wed, Sat, Sun) |
| **Light** | 2 days (Tue, Thu) | 5 |

**Tip:** Keep an eye on your squad's fitness levels. If players' condition drops too low, your physio or assistant manager will send you a warning message. Switch to a **Light** schedule or **Recovery** focus to let them rest before the next match.

### Player Development

Young players (under 21) develop **much faster** than older players. Veteran players (34+) barely grow at all but may already have high attributes. Coaching staff quality also affects growth — better coaches mean faster development.

---

## Managing Staff

Your staff directly impacts your team's training quality:

- **Coaches** — Their coaching attribute determines the training multiplier. Higher coaching = faster player growth. Coaches with a specialization matching your training focus give a 25% bonus.
- **Physio** — Improves condition recovery speed for all players. Essential if you're using intense training.
- **Assistant Manager** — Counts as coaching staff and sends you tactical advice.
- **Scout** — (Future feature: will provide scouting reports on players)

Visit the **Staff** tab to hire free-agent staff or release staff you no longer need.

---

## Match Day

### Pre-Match Setup

When you choose **Go to the Field**, you'll see the Pre-Match screen:

1. **Team Sheet** — Your starting XI and substitutes, based on your formation
2. **Formation** — Make last-minute formation changes
3. **Play Style** — Adjust your tactical approach for this specific match
4. **Set Piece Takers** — Assign your captain, penalty taker, free kick taker, and corner taker

Click **Start Match** when you're ready.

### During the Match

The match plays out minute-by-minute with a live event feed. You'll see:

- **Scoreboard** with current score and minute
- **Possession bar** showing each team's share
- **Event feed** with goals, shots, fouls, cards, and other events
- **Stats tab** with shots, passes, fouls, and cards comparison
- **Lineups tab** with player details

#### Speed Controls

| Control | Effect |
|---------|--------|
| **Pause** | Stop the simulation |
| **Slow** | Slow real-time simulation |
| **Normal** | Standard speed |
| **Fast** | Quick simulation |
| **Instant** | Jump to the next phase boundary |
| **Step** | Advance exactly 1 minute |

#### Making Changes

During the match you can:

- **Substitute players** — Select a player to take off, then choose their replacement from the bench (max 5 substitutions)
- **Change formation** — Switch tactical shape mid-game
- **Change play style** — Adapt your approach based on the score

### Half-Time Break

At half-time, the match pauses and you see:

- **First half summary** — Key events from the first 45 minutes
- **Team talk** — Motivate your players (calm, motivational, assertive, aggressive, praise, or disappointed)
- **Tactical changes** — Adjust formation, play style, and make substitutions

Click **Resume Match** to start the second half.

### Full-Time

After the final whistle:

- **Result summary** with scorers and substitutions made
- **Post-match team talk** — Address the squad based on the result
- **Press Conference** — Answer 3 questions from the media with different response tones

---

## Reading the League

### Standings

Visit the **Tournaments** tab to see the league table. Teams are ranked by:

1. **Points** (3 for a win, 1 for a draw, 0 for a loss)
2. **Goal Difference** (goals scored minus goals conceded)
3. **Goals Scored** (tiebreaker)

### Schedule

The **Schedule** tab shows all your fixtures — past results and upcoming matches. The league uses a double round-robin format: you play every team twice (home and away).

### News

The **News** tab has match reports for every fixture in the league, league roundups after each matchday, and standings updates. It's a good way to keep track of what other teams are doing.

---

## Inbox & Messages

Check your **Inbox** regularly. Messages come from:

- **Board of Directors** — Welcome messages, expectations
- **Assistant Manager / Physio** — Fitness warnings, match previews
- **League Office** — Fixture information
- **System** — Match results and other notifications

Some messages have **action buttons** — for example, a fitness warning might have an "Adjust Training" button that takes you directly to the Training tab.

---

## Tips for New Managers

1. **Check your squad first** — Review player attributes, conditions, and overall ratings before your first match.

2. **Match your tactics to your players** — If your best players are defenders, don't play an Attacking style. If you have fast forwards, Counter might work well.

3. **Don't overtrain** — High intensity + Intense schedule will exhaust your squad. Start with Balanced/Medium and adjust from there.

4. **Use Recovery before big matches** — Switch to Recovery focus the day before a match to ensure your players are fit.

5. **Read your messages** — Your staff gives useful advice about fitness levels and upcoming opponents.

6. **Hire good coaches** — Staff quality directly multiplies your training gains. A coach with a specialization matching your focus gives a significant bonus.

7. **Rotate substitutions** — You have 5 subs per match. Use them to manage fatigue — bring on fresh legs in the second half.

8. **Watch condition levels** — Players below 40% condition will perform noticeably worse. Players below 25% are at risk of injury.

9. **Check the league table** — Know where you stand and how other teams are performing. Adjust your ambitions accordingly.

10. **Save regularly** — The game auto-saves when you exit to the main menu, but you can also save manually from Settings.
