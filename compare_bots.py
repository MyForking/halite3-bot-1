#!/usr/bin/env python3

import sys
import subprocess
import json
import matplotlib.pyplot as plt
from progressbar import ProgressBar
import numpy as np
from sklearn.externals.joblib import Parallel, delayed

def run_game(bot1, bot2):
    x = subprocess.run(['./halite', '--no-replay', '--no-logs', '--results-as-json', '--width 32', '--height 32', bot1, bot2], stdout=subprocess.PIPE)
    result = json.loads(x.stdout)
    return result['stats']['0']['score'], result['stats']['1']['score']

if __name__ == '__main__':

    bot1 = './target/release/my_bot'
    bot2 = './old_bots/v8'

    bot1, bot2 = bot1, bot2

    scores = []
    for _ in ProgressBar()(range(1000)):
        scores.append(run_game(bot1, bot2))

    scores = np.transpose(scores).T
    print(bot1, 'vs', bot2)
    print(np.mean(scores, axis=0))
    print(np.std(scores, axis=0))

    print(np.mean(scores[:, 0] - scores[:, 1]))
    print(np.std(scores[:, 0] - scores[:, 1]))

    plt.hist(scores[:, 0] - scores[:, 1], 33)
    plt.show()
