from copy import deepcopy
import subprocess
import json
import numpy as np
import numba as nb
import matplotlib.pyplot as plt
import datetime
import os
import scipy.stats as sps

def start_game(cfgfile):        
    proc = subprocess.Popen(['../halite', '--no-timeout', '--no-replay', '--no-logs', '--width 32', '--height 32', '--results-as-json', '../target/release/my_bot -c ' + cfgfile, '../old_bots/v13'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return proc

def get_game(proc):    
    proc.wait()
    out, err = proc.communicate()
    #print(out)
    #print(ids, err)
    res = json.loads(out)
    
    score0 = res['stats']['0']['score']
    score1 = res['stats']['1']['score']
    
    return score0, score1, res['map_total_halite']

config = json.load(open('../config.json'))

LEARNING_RATE = 1e-1

parsize = 4
batchsize = 100
n_best = 10

mu = {'ships': {'greedy_pheromone_weight': 1.0,
                'greedy_seek_limit': 50,
                'greedy_harvest_limit': 10,
                'greedy_prefer_stay_factor': 2}}
var = {'ships': {'greedy_pheromone_weight': 10.0,
                'greedy_seek_limit': 100,
                'greedy_harvest_limit': 100,
                'greedy_prefer_stay_factor': 10}}
lim = {'ships': {'greedy_pheromone_weight': (0, 10),
                'greedy_seek_limit': (1, 1000),
                'greedy_harvest_limit': (1, 1000),
                'greedy_prefer_stay_factor': (0, 10)}}
dt = {'ships': {'greedy_pheromone_weight': float,
                'greedy_seek_limit': int,
                'greedy_harvest_limit': int,
                'greedy_prefer_stay_factor': int}}

values = np.logspace(0, -3, 100)
np.random.shuffle(values)

x, y = [], []

for v in values:
    config['ships']['seek_pheromone_cost'] = -v
    
    cfgfile = "tmpcfg.json"
    with open(cfgfile, 'w') as f:
        json.dump(config, f)
        
    procs = [start_game(cfgfile) for i in range(parsize)]
    mean_delta = 0
    for proc in procs:
        s0, s1, tot = get_game(proc)
        mean_delta += (s0 - s1) / tot
    y.append(mean_delta / len(procs))
    x.append(v)
    
    i = np.argsort(x)
    
    plt.semilogx(np.array(x)[i], np.array(y)[i])
    plt.show()
    