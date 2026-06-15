# World Databases

Place `.json` world database files here to make them available when creating a new game.

## Format

```json
{
  "name": "My League",
  "description": "Custom league with 8 teams",
  "teams": [...],
  "players": [...],
  "staff": [...]
}
```

You can export a world database from an existing game using the in-game export feature.

## Optional media paths

Teams and players can include optional local media paths. These paths are generic and dataset-defined; the game does not infer provider-specific URLs or require numeric IDs.

```json
{
  "teams": [
    {
      "id": "team-id",
      "media": {
        "logo": "/assets/worlds/my-world/teams/team-id.png"
      }
    }
  ],
  "players": [
    {
      "id": "player-id",
      "media": {
        "face": "/assets/worlds/my-world/players/player-id.png"
      }
    }
  ]
}
```

Put matching files under `public/assets/worlds/<world-id>/...` for development builds. Missing images fall back to text initials/short names.
