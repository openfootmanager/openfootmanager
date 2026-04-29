# Advanced World Database JSON

This guide explains how to create a structured world with nations, leagues, and clubs.

## What the game supports today

- one manager focused competition (`game.league`) with parallel competitions (`game.leagues`)
- Several competitions in the same fixture calendar through `FixtureCompetition`:
  - `League`
  - `Friendly`
  - `PreseasonTournament`
- Custom countries and football nation codes derived from your world data
- Club to many teams modeling (`club.team_ids`), with each team carrying `club_id` and `team_type`

## Current limitation

- News, context, and some systems does not support parrallel competitions. Nevertheless, everything is working for the main competition.
- Cup progression is currently simplified, there is only a single generated round unless expanded by custom data

## Below, a minimal advanced exemple

```json
{
  "name": "Europe Sandbox",
  "description": "France and Brittany with club structures",
  "countries": [
    { "code": "FR", "name": "France", "league_names": ["Première division française"] },
    { "code": "BZH", "name": "Bretagne", "league_names": ["Kevre Pro"] }
  ],
  "clubs": [
    {
      "id": "club_paris_sg",
      "name": "Paris Saint Germain",
      "country": "France",
      "city": "Paris",
      "team_ids": ["team_psg_first", "team_psg_u19"]
    }
  ],
  "teams": [
    {
      "id": "team_psg_first",
      "club_id": "club_paris_sg",
      "team_type": "FirstTeam",
      "name": "Paris Saint Germain",
      "short_name": "PSG",
      "country": "France",
      "football_nation": "FR",
      "city": "Paris",
      "stadium_name": "Parc des Princes",
      "stadium_capacity": 60000,
      "finance": 7000000,
      "manager_id": null,
      "reputation": 800,
      "wage_budget": 500000,
      "transfer_budget": 2000000,
      "season_income": 0,
      "season_expenses": 0,
      "financial_ledger": [],
      "sponsorship": null,
      "facilities": { "training": 2, "medical": 2, "scouting": 2 },
      "formation": "4-3-3",
      "play_style": "Possession",
      "training_focus": "Physical",
      "training_intensity": "Medium",
      "training_schedule": "Balanced",
      "founded_year": 1899,
      "colors": { "primary": "#cc0000", "secondary": "#ffffff" },
      "training_groups": [],
      "starting_xi_ids": [],
      "match_roles": {
        "captain": null,
        "vice_captain": null,
        "penalty_taker": null,
        "free_kick_taker": null,
        "corner_taker": null
      },
      "form": [],
      "history": []
    }
  ],
  "players": [],
  "staff": []
}
```

## Tips for complex calendars

- keep league teams count even for the round-robin generator, and for the original fixtures generator, later, i will be working on odds support
- use `Friendly` and `PreseasonTournament` fixtures with `matchday: 0` for off-season/preseason blocks
- ensure that fixture dates are on the ISO format (`YYYY-MM-DD`) and chronologically coherent
- league standings only use fixtures where `competition` is `League`

## Definition blueprint (`default_teams.json`)

You can generate large worlds by structure instead of listing all teams manually:

```json
{
  "version": 2,
  "description": "4 nations, 2 leagues each, 10 clubs per league",
  "structure": {
    "nations": [
      {
        "name": "England",
        "code": "ENG",
        "leagues": [
          { "name": "Premier Division", "club_count": 10 },
          { "name": "Championship", "club_count": 10 }
        ]
      }
    ]
  },
  "teams": []
}
```

Rules, already in the definitions.md but maybe what the french people call "a piqure de rappel" is mandatory :
- if `teams` isn't empty, then explicit teams are used
- if `teams` is empty and `structure` is present, then teams are expanded from the blueprint
- Each expanded team gets a `domestic_league` value matching the source league name