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
    proc = subprocess.Popen(['../halite', '--no-timeout', '--no-replay', '--no-logs', '--width 32', '--height 32', '--results-as-json', '../target/release/my_bot ' + aifile, '../target/release/my_bot ' + aifile], stdout=subprocess.PIPE)
    return proc

def get_game(proc):    
    proc.wait()
    out, err = proc.communicate()
    #print(out)
    #print(ids, err)
    res = json.loads(out)
    
    score0 = res['stats']['0']['score'] / res['map_total_halite']
    score1 = res['stats']['1']['score'] / res['map_total_halite']
    
    return score0, score1

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

maxvar = np.inf

score_hist = []
 
n_iter = 0
while n_iter < 100 and maxvar > 1e-2:
    n_iter += 1
    instances = []
    delta_score = []
    for _ in range(batchsize):
        instance = deepcopy(config)
        for cat in mu.keys():
            for val in mu[cat].keys():
                x = np.random.randn() * np.sqrt(var[cat][val]) + mu[cat][val]
                l = lim[cat][val]
                x = np.clip(x, *l)
                instance[cat][val] = dt[cat][val](x)
                
        cfgfile = "tmpcfg.json"
        with open(aifile, 'w') as f:
            json.dump(instance, f)
            
        procs = [start_game(cfgfile) for i in range(parsize)]
        mean_delta = 0
        for proc in procs:
            s0, s1 = get_game(proc)
            mean_delta += s0 - s1
        instances.append(instance)
        delta_score.append(mean_delta / len(procs))
            
    i = np.argsort(delta_score)[-n_best:]
    
    score_hist.append((np.mean(delta_score), np.mean(np.array(delta_score)[i])))
    
    mv = 0
    for cat in mu.keys():
        for val in mu[cat].keys():
            x = [instances[ii][cat][val] for ii in i]
            
            mu[cat][val] = mu[cat][val] * (1-LEARNING_RATE) + LEARNING_RATE * np.mean(x)
            var[cat][val] = var[cat][val] * (1-LEARNING_RATE) + LEARNING_RATE * np.var(x)
            
            mv = max(mv, var[cat][val])
            
    maxvar = min(maxvar, mv)
    
    print(mu)    
    print('{}: max_var={}, scores={}'.format(n_iter, maxvar, score_hist[-1]))
