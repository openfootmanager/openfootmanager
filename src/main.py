from core.championship import Championship
from core.match import Match
from core.player import Player
from core.team import Team

players = [
    ["ter Stegen", 30, 90, "GK"],
    ["Nelson Semedo", 28, 85, "DEF"],
    ["Gerard Pique", 33, 92, "DEF"],
    ["Clement Lenglet", 28, 84, "DEF"],
    ["Jordi Alba", 25, 84, "DEF"],
    ["Ivan Rakitic", 28, 89, "MID"],
    ["Sergio Busquets", 32, 89, "MID"],
    ["Arthur", 32, 88, "MID"],
    ["Luis Suárez", 33, 90, "ATK"],
    ["Lionel Messi", 33, 95, "ATK"],
    ["Dembelé", 29, 85, "ATK"],
    ["Courtois", 28, 93, "GK"],
    ["Marcelo", 33, 88, "DEF"],
    ["Sergio Ramos", 33, 87, "DEF"],
    ["Varane", 28, 89, "DEF"],
    ["Nacho", 28, 88, "DEF"],
    ["Kroos", 29, 90, "MID"],
    ["Modric", 30, 92, "MID"],
    ["Casemiro", 30, 85, "MID"],
    ["James Rodríguez", 22, 82, "MID"],
    ["Hazard", 29, 90, "ATK"],
    ["Balle", 30, 93, "ATK"]
]

teams = []

def initialize_game():
    team = Team("Barcelona", 1)
    teams.append(team)
    team = Team("Real Madrid", 2)
    teams.append(team)

    count = 0

    # Inserting players on each team
    for player in players:
        if count <= 10:
            i = 0
        else:
            i = 1

        name = player[0]
        age = player[1]
        skill = player[2]
        pos = player[3]

        pl = Player(count, name, age, skill, pos, 100)
        teams[i].players.append(pl)
        count += 1
    
def main():
    initialize_game()
        

if __name__ == "__main__":
    main()