#!/usr/bin/env python3

import sys
import subprocess
import json
import matplotlib.pyplot as plt
from progressbar import ProgressBar
import numpy as np
from sklearn.externals.joblib import Parallel, delayed

def run_game(bot1, bot2, s):
    x = subprocess.run(['./halite', '--no-replay', '--no-logs', '--results-as-json', '--width '+s, '--height ' + s, bot1, bot2], stdout=subprocess.PIPE)
    result = json.loads(x.stdout)
    return result['stats']['0']['score'], result['stats']['1']['score']

if __name__ == '__main__':

    bot1 = './target/release/my_bot'
    bot2 = './old_bots/v13'

    bot1, bot2 = bot1, bot2

    s = '32'

    n = 100

    scores = []
    for _ in ProgressBar()(range(n//2)):
        scores.append(run_game(bot1, bot2, s))
        scores.append(run_game(bot2, bot1, s)[::-1])

    scores = np.transpose(scores).T
    print(bot1, 'vs', bot2)
    print(np.mean(scores, axis=0))
    print(np.std(scores, axis=0))

    print(np.median(scores[:, 0] - scores[:, 1]))
    print(np.mean(scores[:, 0] - scores[:, 1]))
    print(np.std(scores[:, 0] - scores[:, 1]))

    plt.hist(scores[:, 0] - scores[:, 1], int(np.sqrt(n)))
    plt.show()
