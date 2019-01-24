# Halite III Bot

This repository tracks the code I wrote for the [Halite Challenge](https://halite.io/).

> Halite is an open source artificial intelligence challenge, created by Two Sigma.
>
>Halite III is a resource management game. Your goal is to build a bot that efficiently navigates the seas collecting halite, a luminous energy resource.

The code is an ugly unstructured mess that grew over time. It is peppered with magic numbers and sprinkled with inconsistencies. I always wanted to clean it up but then rather spent the time optimizing the bot by making the code even uglier.

If you have any questions or comments please don't hesitate to post an issue at the [Github repository](https://github.com/mbillingr/halite3-bot/issues).

## Why Rust?

The primary reason I wrote this bot in Rust is that I wanted to. I use Rust for all my recent hobby projects for that reason. Python and numpy might have made rapid development of new features easier, but I think Rust was still a good choice for this project:
1. There was no time pressure. I joined in late, but still had a month of development left. If the [borrow checker fought me](https://m-decoster.github.io/2017/01/16/fighting-borrowchk/) I could simply stay up an hour longer on that evening until it calmed down again.
2. If my code compiled and worked at home I was confident it would compile and work correctly on the servers too. In Python there is always that typo waiting in that function that was never entered during local testing, and in C++ I always get sloppy with RAII and pointers, then spend the rest of the evening tracking that elusive segfault.

## Initial plans and what I actually ended up with

I found out about the Halite Challenge late in December while researching something for an [Advent of Code](https://adventofcode.com/2018) puzzle. Needless to say there was no way to resist joining in.

Initially, I had big plans. I wanted to create a basic scripted AI framework and replace it step by step with machine-learned components. This plan failed spectacularly. I did not even manage to train a simple neural network for greedy ship navigation based on the surrounding 3x3 grid. Neither could I get automatic optimization of configuration parameters to work. I ended up with fully hand-written logic and <s>arbitrarily chosen</s> manually tuned hyperparameters.

In hindsight, I suspect the reason for my machine learning failure was that I went for the wrong criterion. I always tried to maximize the amount of halite collected, which is a super volatile measure. Even when correcting for the total amount of halite in the map the game to game variance is so high that it must have dwarved any learning effects. I should probably have optimized the win rate instead... that's what you get for trying to be smarter than reinforcement learning tutorials :)

### Details of the Final Version

These are the main components of the AI bot:

- A [pheromone simulation](https://en.wikipedia.org/wiki/Fick%27s_laws_of_diffusion#Fick's_second_law_2)
- A [pushdown automaton](https://en.wikipedia.org/wiki/Pushdown_automaton) controls each individual ship
- Another pushdown automaton (overengineered; it only has a single state) controls the virtual commander
- A [movement solver](https://en.wikipedia.org/wiki/Hungarian_algorithm)
- An opponent movement predictor

#### Pheromone simulation

Pheromones spread by diffusion over the game map and evaporate slowly when not replenished. They are used by individual ships for navigation and decision making. My initial idea was to have the ships behave like ants, leaving a pheromone trail while carrying yummy halite back to the base. However, I soon realized that my bot could do better. Ants do this because they lack global information about their environment, but in Halite we have all the information we need. So in addition to letting the ships emit pheromones I made the environment emit pheromones. In particular, each map cell emits pheromones based on the amount of halite it contains.

Ships tended to ball up around halite-rich locations, which I prevented by making ships absorb pheromones based on their free cargo space. Having ships that return to base emit pheromones turned out to improve bot performance somewhat.

Also, opponent ships emit a small amount of pheromones the more halite they carry. When the map runs out of halite overall pheromone concentration drops and ships tend to follow opponent ships, waiting for an opportunity to sink them.

#### Movement Solver

Movement conflicts are solved in the framework of the [Assignment Problem](https://en.wikipedia.org/wiki/Assignment_problem): Each ship assigns a cost to each of the 5 positions it could move to. The [Khun-Munkres](https://en.wikipedia.org/wiki/Hungarian_algorithm) algorithm solves the assignment of every ship to exactly one position so that the overall cost is minimized. A ship that cannot move because it does not have enough halite sets movement costs to infinity and stay-still-cost to 0. Spawning a new ship is treated as a special "move" with -infinity cost at the shipyard position.

This works great. Ships with low-priority tasks move out of the way of ships with high-priority tasks, and seemingly efficient movement patterns emerge automatically. There are almost no traffic jams and no accidental collisions.

#### Commander AI

The commander coordinates building of dropoff structures, spawning of new ships, and assigning new tasks to ships on request.

Task assignment is simple. If a ship has less than 500 halite loaded (practically, that's usually 0) it is assigned the `Collect` task, and otherwise it is assigned the `Deliver` task to return the halite back to base.

The commander spawns a new ship if enough halite is available, the amount of halite left on the map per ship currently alive is higher than the ship construction cost, and there are at least `map_width * 2` rounds left in the game.

The conditions for building a dropoff are
- the adaptive average of turns taken by ships to return their cargo exceeds 10
- there exists a location with halite density >= 100 (map halite averaged over a manhatten-radius of 5)

If these conditions are met, the commander looks for a ship that satisfies the following criteria:
- manhatten distance to nearest own structure >= 15
- at least 3 friendly ships within a radius of 12
- halite density at ship's location >= 100
If multiple ships satisfy these criteria, the ship seeing the highest pheromone concentration at its current location is instructed to build a dropoff.

#### Ship AI

Ship AI is based on a pushdown automaton, which allows ships to temporarily take on a new task and then continue with what they did before. For example, a ship currently collecting might be instructed to build a dropoff. If that fails it simply resumes collecting. If a ship runs out of tasks it requests a new task from the commander.

The ship AI knows four different tasks (states): `Collect`, `Deliver`, `GoHome`, `BuildDropoff`. The latter is the simplest task. The ship tries to build a dropoff at its current location. `Deliver` and `GoHome` are very similar. Both let the ship follow the cheapest path to a dropoff point. `Deliver` finishes when the ship's cargo hold is empty and ships try to avoid all positions reachable by enemy ships in the hope of countering simple battle tactics. In contrast `GoHome` is used at the end of the game. Ships do not care about enemies' battle tactics because presumably they are busy returning home themselves.

`Collect` is the most complex state. It contains harvesting and battle logic, and decides when it's time to `GoHome`. In principle, ships want to climb the pheromone gradient, unless their current position contains enough halite. If they lack a useful gradient, they simply try to move away from friendly structures. Ships that carry more than 500 halite try to avoid positions reachable by opponent ships.

Finally, the opportunistic battle logic can override the previous `Collect` rules. *Opportunistic* means that it kicks in if an opponent ship happens to be adjacent to a ship in `Collect` state. If the opponent ship carries more halite than our ship we find the distance `r` to the next nearest opponent ship. If the total amount of free cargo space of all friently ships within `r` steps can take enough halite our ship considers to move in for the kill. It scales the cargo difference with an *aggression* constant (1000 in 2p games, 10 in 4p games) and uses this value to weigh the move.


#### Movement Predictor

The movement predictor actually does not predict very much. I had bigger plans but ran out of time :)

It simply classifies all map positions as `Clear`, `Occupied` by an opponenent, and `Reachable` by an opponent. This information is used by the ship AI to weigh their moves based on their safety needs.


### Known Bugs in the Final Version

- Ships refuse to return to enter the dropoff/shipyard for unloading if an opponent ship is adjacent to the structure. ([`movement_predictor.rs:75`](https://github.com/mbillingr/halite3-bot/blob/master/src/movement_predictor.rs#L75) wrongly uses the ship position instead of the adjacent tile position.)


## Table of Features and Ranks

The table below shows the ranks (second row) achieved by each bot version (first row). Additionally, `###` indicates that a feature was implemented in a specific version.

```
| Feature        bot version | <=4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 |
|----------------------------|-----|---|---|---|---|---|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|
|                       rank |~1000|1236|666|644|328|361|253| 231| 224| 192| 199| 190| 120| 141| 109| 114| 84 | 82 | 71 |  86|  52|  ? |
| AI Finite State Machine    | ####|###|###|###|###|   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |
| AI Behavior Trees          |     |   |   |   |   |###|####|####|####|####|####|####|####|####|####|####|####|    |    |    |    |    |
| Ai Pushdown Automaton      |     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |####|####|####|####|####|
| Pheromone Engine           |     |   |   |   |   |   |    |    |    |####|####|####|####|####|####|####|####|####|####|####|####|####|
| Movement Solver            |     |   |   |   |   |   |    |    |    |    |####|####|####|####|####|####|####|####|####|####|####|####|
| Build Dropoffs             |     |   |   |   |   |   |    |####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|
| Greedy Gathering           | ####|###|###|###|###|###|####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|
| Use Inspiration            |     |   |   |   |   |   |    |    |    |    |    |####|####|####|####|####|####|####|####|####|####|####|
| Search Gathering locations |     |   |   |   |###|###|####|####|####|####|    |    |    |    |    |    |    |    |    |    |    |    |
| Low-Halite Gathering       |     |   |   |   |   |   |####|####|####|    |    |    |    |    |    |    |    |    |    |    |    |    |
| Ships seek halite maxima   | ####|###|   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |
| Deliver Halite: naive nav. | ####|   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |
| Deliver Halite: Dijkstra   |     |###|###|###|###|###|####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|
| Dijkstra: length penalty   |     |   |   |   |   |   |    |    |    |    |    |    |####|####|####|####|####|####|####|####|####|####|
| End of Game Returning Home |     |   |   |   |   |   |####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|
| Spawn based on performance | ####|###|###|###|###|###|####|####|####|####|####|####|####|####|####|####|    |    |    |    |    |    |
| Spawn based on halite left |     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |####|####|####|####|####|####|
| Spawn if enemy on shipyard | ####|###|###|###|###|###|####|####|####|####|####|####|####|    |    |    |    |    |    |    |    |    |
| Kamikaze opponents on s.y. |     |   |   |###|###|###|####|####|####|####|####|####|####|    |    |    |    |    |    |    |    |    |
| Ignore opponents on s.y.   |     |   |   |   |   |   |    |    |    |    |    |    |    |####|####|####|####|####|####|####|####|####|
| Force move ships from s.y. |     |   |   |   |###|###|####|    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |
| Multiple ships from s.y.   |     |   |   |   |   |   |####|####|####|####|    |    |    |    |    |    |    |    |    |    |    |    |
| Enough halite to move?     |     |   |   |   |   |   |####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|####|
| Ignore opponent ships      |     |   |   |   |   |   |    |    |    |####|####|####|####|    |    |    |    |    |    |    |    |    |
| Avoid all opponent contact |     |   |   |   |   |   |    |    |    |    |    |    |    |####|    |    |    |    |    |    |    |    |
| Avoid opponent ship pos.   | ####|###|###|###|###|###|####|####|####|    |    |    |    |####|####|####|####|####|####|####|####|####|
| Collide high-cargo opponents|    |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |####|####|####|####|####|
| High-cargo ships flee battles|   |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |####|####|####|####|
| Opponents emit pheromones  |     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |####|####|
| Return earlier in late game|     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |####|####|
```
