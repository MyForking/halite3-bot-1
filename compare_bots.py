#!/usr/bin/env python3

import sys
import subprocess
import json
import matplotlib.pyplot as plt
from progressbar import ProgressBar
import numpy as np
import random
from sklearn.externals.joblib import Parallel, delayed

def run_game(bots, s):
    x = subprocess.run(['./halite', '--no-replay', '--no-logs', '--results-as-json', '--width '+s, '--height '+s] + bots, stdout=subprocess.PIPE)
    result = json.loads(x.stdout)
    return [result['stats'][str(i)]['score'] for i in range(len(bots))]

if __name__ == '__main__':

    bots = ['./target/release/my_bot',
            #'./old_bots/v16 -c old_bots/v16.cfg.json',
            #'./old_bots/v16 -c old_bots/v16.cfg.json',
            './old_bots/v16 -c old_bots/v16.cfg.json']

    size = '32'

    n = 100

    scores_list = []
    n_wins = 0
    for k in ProgressBar()(range(n)):
        order = list(range(len(bots)))
        random.shuffle(order)
        s = run_game([bots[i] for i in order], size)
        n_wins += np.argmax(s) == order.index(0)
        scores_list.append((s[order.index(0)], np.mean([s[order.index(i)] for i in order[1:]])))

        if (k + 1) % 10 == 0:
            scores = np.array(scores_list)
            print(bots[0], 'vs', bots[0])
            print(np.mean(scores, axis=0))
            print(np.std(scores, axis=0))

            print(np.median(scores[:, 0] - scores[:, 1]))
            print(np.mean(scores[:, 0] - scores[:, 1]))
            print(np.std(scores[:, 0] - scores[:, 1]))
            print(n_wins, 'wins')

    plt.hist(scores[:, 0] - scores[:, 1], int(np.sqrt(n)))
    plt.show()
