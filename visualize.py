#!/bin/env python3

import numpy as np
import matplotlib.pyplot as plt
import json


halite_maps = []
pheros_maps = []
ships = []


with open('dump.json') as f:
    data = f.read()

    for turn in data.split('\n===\n')[:-1]:
        dump = json.loads(turn)

        W = dump['game']['map']['width']
        H = dump['game']['map']['height']

        halite_maps.append(np.array([cell['halite'] for row in dump['game']['map']['cells'] for cell in row]).reshape((H, W)))

        pheros_maps.append(np.array([cell for row in dump['pheromones'] for cell in row]).reshape((H, W)))

        ships.append(np.array([(ship['position']['x'], ship['position']['y'], ship['owner']) for ship in dump['game']['ships'].values()]).reshape(-1, 3))

        pheros_maps[-1] -= np.min(pheros_maps[-1])
        print(np.max(pheros_maps[-1]))

        phe = np.stack([np.ones_like(pheros_maps[-1]) * 0.5,
                        np.ones_like(pheros_maps[-1]) * 0.2,
                        np.ones_like(pheros_maps[-1]) * 0.6,
                        pheros_maps[-1] / np.max(pheros_maps[-1])]).transpose(1, 2, 0)

        print(phe.shape)

        #plt.imshow(halite_maps[-1], vmin=0, vmax=1000, cmap='bone')
        plt.imshow(phe, vmin=0, vmax=1000, cmap='bone')

        plt.scatter(ships[-1][:, 0], ships[-1][:, 1], c=ships[-1][:, 2])

        plt.show()