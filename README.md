# Halite III Bot

This repository tracks the code I wrote for the [Halite Challenge](https://halite.io/).

> Halite is an open source artificial intelligence challenge, created by Two Sigma.
>
>Halite III is a resource management game. Your goal is to build a bot that efficiently navigates the seas collecting halite, a luminous energy resource.

The code is an ugly unstructured mess that grew over time. It is peppered with magic numbers and sprinkled with inconsistencies. I always wanted to clean it up but then rather spent the time optimizing the bot by making the code even uglier.


## Why Rust?

The primary reason I wrote this bot in Rust is that I wanted to. I use Rust for all my recent hobby projects for that reason. Python and numpy might have made rapid development of new features easier, but I think Rust was still a good choice for this project:
1. There was no time pressure. I joined in late, but still had a month of development left. If the borrow checker fought me I could simply stay up an hour longer on that evening until it calmed down again.
2. If my code compiled and worked at home I was confident it would compile and work correctly on the servers too. In Python there is always that typo waiting in that function that was never entered during local testing, and in C++ I always get sloppy with RAII and pointers, saying *hello* to mr. segfault.

## Initial plans and what I actually ended up with

to do


## Known Bugs in the Final Version

- Ships refuse to return to enter the dropoff/shipyard for unloading if an opponent ship is adjacent to the structure. ([`movement_predictor.rs:75`](https://github.com/mbillingr/halite3-bot/blob/master/src/movement_predictor.rs#L75) wrongly uses the ship position instead of the adjacent tile position.)


## Table of Features and Ranks

The table below shows the ranks (second row) achieved by each bot version (first row). Additionally, a :x: indicates if a feature was implemented in that version.

|Feature                     | <=4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 |
|----------------------------|-----|---|---|---|---|---|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|
|                            |~1000|1236|666|644|328|361|253| 231| 224| 192| 199| 190| 120| 141| 109| 114| 84 | 82 | 71 |  86|  52|  ? |
| AI State Machine           | :x: |:x:|:x:|:x:|:x:|
| AI Behavior Trees          |     |   |   |   |   |:x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Ai Pushdown Automaton      |     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    | :x:| :x:| :x:| :x:| :x:|
| Pheromone Engine           |     |   |   |   |   |   |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| [Kuhn-Munkres](https://en.wikipedia.org/wiki/Hungarian_algorithm) movement solver |     |   |   |   |   |   |    |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Build Dropoffs             |     |   |   |   |   |   |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Greedy Gathering           | :x: |:x:|:x:|:x:|:x:|:x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Use Inspiration            |     |   |   |   |   |   |    |    |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Search Gathering locations |     |   |   |   |:x:|:x:| :x:| :x:| :x:| :x:|
| Low-Halite Gathering       |     |   |   |   |   |   | :x:| :x:| :x:|
| Some Ships seek halite maxima |:x:|:x:|
| Deliver Halite: naive nav. | :x: |
| Deliver Halite: Dijkstra   |     |:x:|:x:|:x:|:x:|:x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Dijkstra: path length penalty|   |   |   |   |   |   |    |    |    |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| End of Game Returning Home |     |   |   |   |   |   | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Spawn based on performance | :x: |:x:|:x:|:x:|:x:|:x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Spawn based on halite left |     |   |   |   |   |   |    |    |    |    |    |    |    |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:|
| Spawn if enemy on shipyard | :x: |:x:|:x:|:x:|:x:|:x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Kamikaze if opponent on s.y.|    |   |   |:x:|:x:|:x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Simply ignore opponents on my structures|||| |   |   |    |    |    |    |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Force move ships from structures | | |   |   |:x:|:x:| :x:|
| Multiple ships from structures   | | |   |   |   |   | :x:| :x:| :x:| :x:|
| Enough halite to move?     |     |   |   |   |   |   | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Ignore opponent ships      |     |   |   |   |   |   |    |    |    | :x:| :x:| :x:| :x:|
| Avoid all opponent contact |     |   |   |   |   |   |    |    |    |    |    |    |    | :x:|
| Avoid opponent ship positions|:x:|:x:|:x:|:x:|:x:|:x:| :x:| :x:| :x:|    |    |    |    | :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:| :x:|
| Sink adjacent high-cargo opponents|| |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    | :x:| :x:| :x:| :x:| :x:|
| High-cargo ships avoid opponents||   |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    | :x:| :x:| :x:| :x:|
| Opponents emit pheromones  |    |    |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    | :x:| :x:|
| Ships return earlier in late game||  |   |   |   |   |    |    |    |    |    |    |    |    |    |    |    |    |    |    | :x:| :x:|
