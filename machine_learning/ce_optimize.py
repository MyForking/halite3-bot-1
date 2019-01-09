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
    
    score0 = 100 * res['stats']['0']['score'] / res['map_total_halite']
    score1 = 100 * res['stats']['1']['score'] / res['map_total_halite']
    
    return score0, score1

config = json.load(open('../config.json'))

LEARNING_RATE = 3e-1

parsize = 4
batchsize = 1000
n_best = 100

mu = {'ships': {'seek_pheromone_cost': -2}}
var = {'ships': {'seek_pheromone_cost': 1.0}}
lim = {'ships': {'seek_pheromone_cost': (-np.inf, np.inf)}}
trans = {'ships': {'seek_pheromone_cost': lambda x: -(10**x)}}
inv = {'ships': {'seek_pheromone_cost': lambda y: np.log10(-y)}}

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
                instance[cat][val] = trans[cat][val](x)
                
        cfgfile = "tmpcfg.json"
        with open(cfgfile, 'w') as f:
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
            x = np.array([instances[ii][cat][val] for ii in i])
            x = inv[cat][val](x)
            
            mu[cat][val] = mu[cat][val] * (1-LEARNING_RATE) + LEARNING_RATE * np.mean(x)
            var[cat][val] = var[cat][val] * (1-LEARNING_RATE) + LEARNING_RATE * np.var(x)
            
            mv = max(mv, var[cat][val])
            
    maxvar = min(maxvar, mv)
    
    print(mu)    
    print('{}: max_var={}, scores={}'.format(n_iter, maxvar, score_hist[-1]))
