# Halite III Bot

This repository tracks the code I wrote for the [Halite Challenge](https://halite.io/).

> Halite is an open source artificial intelligence challenge, created by Two Sigma.
>
>Halite III is a resource management game. Your goal is to build a bot that efficiently navigates the seas collecting halite, a luminous energy resource.

The code is an ugly unstructured mess that grew over time. It is peppered with magic numbers and sprinkled with inconsistencies. I always wanted to clean it up but then rather spent the time optimizing the bot by making the code even uglier.


## Why Rust?

The primary reason I wrote this bot in Rust is that I wanted to. I use Rust for all my recent hobby projects for that reason. Python and numpy might have made rapid development of new features easier, but I think Rust was still a good choice for this project:
1. There was no time pressure. I joined in late, but still had a month of development left. If the [borrow checker fought me](https://m-decoster.github.io/2017/01/16/fighting-borrowchk/) I could simply stay up an hour longer on that evening until it calmed down again.
2. If my code compiled and worked at home I was confident it would compile and work correctly on the servers too. In Python there is always that typo waiting in that function that was never entered during local testing, and in C++ I always get sloppy with RAII and pointers, saying *hello* to mr. segfault.

## Initial plans and what I actually ended up with

to do


## Known Bugs in the Final Version

- Ships refuse to return to enter the dropoff/shipyard for unloading if an opponent ship is adjacent to the structure. ([`movement_predictor.rs:75`](https://github.com/mbillingr/halite3-bot/blob/master/src/movement_predictor.rs#L75) wrongly uses the ship position instead of the adjacent tile position.)


## Table of Features and Ranks

The table below shows the ranks (second row) achieved by each bot version (first row). Additionally, `###` indicates that a feature was implemented in a specific version.

```
| Feature        bot version | <=4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 |
|----------------------------|-----|---|---|---|---|---|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|
|                       rank |~1000|1236|666|644|328|361|253| 231| 224| 192| 199| 190| 120| 141| 109| 114| 84 | 82 | 71 |  86|  52|  ? |
| AI State Machine           | ####|###|###|###|###|   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |    |
| AI Behavior Trees          |     |   |   |   |   |###|####|####|####|####|####|####|####|####|####|####|####|    |    |    |    |    |
| Ai Pushdown Automaton      |     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |####|####|####|####|####|
| Pheromone Engine           |     |   |   |   |   |   |    |    |    |####|####|####|####|####|####|####|####|####|####|####|####|####|
| Movement Solver (*)        |     |   |   |   |   |   |    |    |    |    |####|####|####|####|####|####|####|####|####|####|####|####|
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
| Leturn earlier in late game|     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    |####|####|
```

(*) [Kuhn Munkres Algorithm](https://en.wikipedia.org/wiki/Hungarian_algorithm)
