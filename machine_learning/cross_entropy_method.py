import subprocess
import json
import numpy as np
import numba as nb
import matplotlib.pyplot as plt
from datetime import datetime
from sklearn.externals.joblib import Parallel, delayed


MOVE_FACTOR = 10
GAIN_FACTOR = 4


def get_map():
    x = subprocess.run(['../halite', '--no-replay', '--no-logs', '--width 32', '--height 32', '--results-as-json', 'true', 'true'], capture_output=True)
    res = json.loads(x.stdout)
    w = res['map_width']
    h = res['map_height']
    x = res['final_snapshot'].split(',')[4:][:w*h]
    x[0] = x[0].split(';')[1]
    return np.reshape([int(x) for x in x], [w, h])

@nb.jit(nopython=True)
def softmax(x):
    """
    Compute softmax values for each sets of scores in x.
    
    Rows are scores for each class. 
    Columns are predictions (samples).
    """
    shiftx = x - np.max(x)
    exps = np.exp(shiftx)
    return exps / np.sum(exps)


class Agent:
    def __init__(self, n_input, n_hidden):
        #ws = np.random.randn(n_hidden, *input_shape) / np.sqrt(np.prod(input_shape))
        #w1 = np.random.randn(n_hidden, *input_shape) / np.sqrt(np.prod(input_shape))
        #w2 = np.random.randn(n_hidden) / np.sqrt(n_hidden)
        #self.set_weights(ws, w1, w2)
        self.w1 = np.random.randn(n_hidden, n_input) / np.sqrt(np.prod(n_input))
        self.w2 = np.random.randn(5, n_hidden) / np.sqrt(n_hidden)
    
    @nb.jit
    def forward_step(self, x):
        return forward_step(x, self.w1, self.w2)
    
    @nb.jit
    def backward_step(self, nx, nh, ndlogp):
        """nx, nh and nlogp are stacks of collected hidden states and gradient-logprobs"""
        dw2 = np.einsum('nh,nk->kh', nh, ndlogp)
        dh = np.einsum('nk,kh->nh', ndlogp, self.w2)
        dh[nh <= 0] = 0
        dw1 = np.einsum('nh,ni->hi', dh, nx)
        return dw1, dw2
    
    def dump(self, filename):
        w1 = self.w1.reshape(self.w1.shape[0], -1)
        data = {'n_input': w1.shape[1],
                'n_hidden': w1.shape[0],
                'n_output': w2.shape[0],
                'layers': [{'weights': w1.ravel(), 'activation': 'relu'},
                           {'weights': self.w2.ravel(), 'activation': 'linear'}]}
        with open(filename) as fp:
            json.dump(data, fp)
            
    def export(self):
        w1 = self.w1.reshape(self.w1.shape[0], -1)
        print('n_input:', w1.shape[1])
        print('n_hidden:', w1.shape[0])
        print('n_output:', self.w2.shape[0])
        print('layer1_weights: vec![{}],'.format(', '.join('{}'.format(w) for w in w1.ravel())))
        print('layer2_weights: vec![{}],'.format(', '.join('{}'.format(w) for w in self.w2.ravel())))
              
    
@nb.jit
def forward_step(x, w1, w2):
    x = np.concatenate([x, [1]])
    h = np.maximum(0, np.einsum('hi,i->h', w1, x))  # hidden layer relu activations
    h = np.concatenate([h, [1]])
    logp = np.einsum('kh,h->k', w2, h)
    return logp, h
    
@nb.jit
def run_agent(n_steps, pos, w1, w2, mapdata):
    collected = 0
    
    path = np.empty((n_steps, 2))
    
    for i in range(n_steps):
        pos = width + pos % width
            
        cost = np.floor(mapdata[pos[0], pos[1]] / 10)
        if collected < cost:
            action = 0
        else:            
            x = mapdata[pos[0]-r:pos[0]+r+1, pos[1]-r:pos[1]+r+1].ravel() / 1000
            logp, h = forward_step(x, w1, w2)
            action = np.argmax(logp)
        
        if action == 0:
            amount = np.ceil(mapdata[pos[0], pos[1]] / 4)
            mapdata[pos[0], pos[1]] -= amount
            collected += amount
        else:
            collected -= cost
            if action == 1:
                pos[0] += 1
            elif action == 2:
                pos[1] += 1
            elif action == 3:
                pos[0] -= 1
            elif action == 4:
                pos[1] -= 1
                
        path[i] = pos
    return collected, path
    
    
mapdata = get_map()    
#mapdata = (np.linspace(0, 1000, 15)[:, None] * np.ones((1, 15))).T
#mapdata = np.ones((15, 15)) * 1000

width, height = mapdata.shape
assert width == height
mapdata0 = np.tile(mapdata, [3, 3])

LEARNING_RATE = 1e-1

batchsize = 1000
n_best = 100

r = 2
n_steps = 5

n_input = ((r*2+1)**2)
n_hidden = 25
n_output = 5

mu_w1 = np.zeros((n_hidden, n_input + 1))
mu_w2 = np.zeros((n_output, n_hidden + 1))

var_w1 = np.ones_like(mu_w1) * 10000
var_w2 = np.ones_like(mu_w2) * 10000

mean_s = []

#mapdata_backup = mapdata0.copy()
for it in range(100000):
    #mapdata = get_map()
    #width, height = mapdata.shape
    #mapdata0 = np.tile(mapdata, [3, 3])
    
    mapdata0 = (mapdata_backup * np.random.rand()).astype(int)
    
    pos = np.random.randint(width, 2*width, 2)
    
    s = []
    all_w1 = np.random.randn(batchsize, *mu_w1.shape) * np.sqrt(var_w1) + mu_w1
    all_w2 = np.random.randn(batchsize, *mu_w2.shape) * np.sqrt(var_w2) + mu_w2
    for w1, w2 in zip(all_w1, all_w2):
        collected, _ = run_agent(n_steps, pos.copy(), w1, w2, mapdata0.copy())        
        s.append(collected)
    
    i = np.argsort(s)[-n_best:]
    
    mu_w1 = mu_w1 * (1-LEARNING_RATE) + LEARNING_RATE * np.mean(all_w1[i], axis=0)
    mu_w2 = mu_w2 * (1-LEARNING_RATE) + LEARNING_RATE * np.mean(all_w2[i], axis=0)
    
    var_w1 = var_w1 * (1-LEARNING_RATE) + LEARNING_RATE * np.var(all_w1[i], axis=0)
    var_w2 = var_w2 * (1-LEARNING_RATE) + LEARNING_RATE * np.var(all_w2[i], axis=0)
    
    mean_s.append(np.mean(s) / np.mean(mapdata0))
        
    if it % 10000 == 9999:
        plt.plot(mean_s)
        plt.show()
        
mapdata = mapdata0.copy()
collected, path = run_agent(n_steps * 10, pos.copy(), mu_w1, mu_w2, mapdata)
plt.imshow(mapdata)
plt.colorbar()
    
rgb = np.stack([np.linspace(1, 1, len(path)),
    np.linspace(0.5, 0, len(path)),
    np.linspace(0, 0.5, len(path))]).T

for i, ((y, x), c) in enumerate(zip(path, rgb)):
    plt.plot(x, y, '.', color=c)
    
mid = np.mean(path, axis=0)

diff = max(np.max(path[:, 0]) - np.min(path[:, 0]), np.max(path[:, 1]) - np.min(path[:, 1]), 8)
    
plt.ylim(mid[0] - diff//2, mid[0] + diff//2)
plt.xlim(mid[1] - diff//2, mid[1] + diff//2)
    
        