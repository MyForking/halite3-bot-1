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
    h = np.maximum(0, np.einsum('hi,i->h', w1, x))  # hidden layer relu activations
    logp = np.einsum('kh,h->k', w2, h)
    return logp, h
    
@nb.jit
def run_agent(n_steps, pos, w1, w2, mapdata, deterministic=False):
    collected = 0
    
    xs = np.empty((n_steps, w1.shape[1]))
    hs = np.empty((n_steps, w1.shape[0]))
    ps = np.empty((n_steps, 5))
    rs = np.empty((n_steps, 1))
    
    path = np.empty((n_steps, 2))
    
    for i in range(n_steps):
        pos = width + pos % width
        x = mapdata[pos[0]-r:pos[0]+r+1, pos[1]-r:pos[1]+r+1].ravel() / 1000
        x = np.concatenate([[1], x])
        logp, h = forward_step(x, w1, w2)
        p = softmax(logp)
        if deterministic:
            action = np.argmax(p)
        else:
            action = np.random.choice(5, p=p)
        y = np.zeros(5); y[action] = 1
            
        cost = np.floor(mapdata[pos[0], pos[1]] / 10)
        if collected < cost:
            action = 0
        
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
            
        xs[i] = x
        hs[i] = h
        ps[i] = y - p
    return collected, xs, hs, ps, path

@nb.jit(nopython=True)
def move(pos, d, mapdata):
    H, W = mapdata.shape
    if d == 4:
        return (pos[0] % H, (pos[1] - 1) % W)
    if d == 2:
        return (pos[0] % H, (pos[1] + 1) % W)
    if d == 3:
        return ((pos[0] - 1) % H, pos[1] % W)
    if d == 1:
        return ((pos[0] + 1) % H, pos[1] % W)
    return pos
    
@nb.jit(nopython=True)
def run_greedy(n_steps, pos, mapdata):
    collected = 0
    
    for i in range(n_steps):        
        move_cost = np.floor(mapdata[pos] / MOVE_FACTOR)
        stay_gain = np.ceil(mapdata[pos] / GAIN_FACTOR)
        
        if collected < move_cost:
            d = 0
        else:        
            best = (stay_gain, 0)
            order = np.array([1, 2, 3, 4])
            np.random.shuffle(order)
            
            for d in order:
                p = move(pos, d, mapdata)
                move_value = np.ceil(mapdata[p] / GAIN_FACTOR) - move_cost
                if move_value > best[0]:
                    best = (move_value, d)
            d = best[1]
            
        if d == 0:
            mapdata[pos] -= stay_gain
            collected += stay_gain
        else:
            pos = move(pos, d, mapdata)
            collected -= move_cost
            
    return collected
    
    
mapdata = get_map()    
#mapdata = (np.linspace(0, 1000, 15)[:, None] * np.ones((1, 15))).T
#mapdata = np.ones((15, 15)) * 1000

width, height = mapdata.shape
assert width == height
mapdata0 = np.tile(mapdata, [3, 3])

LEARNING_RATE = 1e-2
DECAY_RATE = 0.9
L1_FACTOR = LEARNING_RATE * 0.01

batchsize = 100
r = 2
n_steps = 5
agent = Agent(((r*2+1)**2)+1, 25)

a = np.load('ai4d.npz')
agent.w1 = a['w1']
agent.w2 = a['w2']
del a


#ws = np.zeros((1, 5, 5))
#ws[0, 2, 2] = 0.1
#w1 = np.zeros((1, 5, 5))
#w1[0, 3, 2] = 0.1
#w2 = [1.0]
#agent.set_weights(ws, w1, w2)

rmsprop_dw1 = np.zeros_like(agent.w1)
rmsprop_dw2 = np.zeros_like(agent.w2)

mean_reward = []
all_w1 = []
all_w2 = []

batch = 0
counter = 8
upda = 0
noup = 0
while True:
    mapdata = get_map()    
    width, height = mapdata.shape
    mapdata0 = np.tile(mapdata, [3, 3])
    
    if counter < 1000:
        counter *= 2
    
    for _ in range(counter):
        batch += 1
        xs, hs, ps, rs = [], [], [], []
        
#        for _ in range(batchsize):
#            collected, x, h, p = run_agent(n_steps, pos.copy(), agent.w1, agent.w2, mapdata0.copy())
#            rs.extend([collected] * n_steps)
#            xs.extend(x)
#            hs.extend(h)
#            ps.extend(p)
        
        pos = np.random.randint(width, 2*width, 2)
        
        results = [run_agent(n_steps, pos.copy(), agent.w1, agent.w2, mapdata0.copy()) for _ in range(batchsize)]
        #results = Parallel(n_jobs=5)(delayed(run_wrap)(n_steps, pos.copy(), agent.w1, agent.w2, mapdata0.copy()) for _ in range(batchsize))
        for collected, x, h, p, _ in results:
            ref  = run_greedy(n_steps, tuple(pos), mapdata0.copy())
            rs.extend([collected - ref] * n_steps)
            xs.extend(x)
            hs.extend(h)
            ps.extend(p)
        
        xs = np.array(xs)
        hs = np.array(hs)
        ps = np.array(ps)
        rs = np.array(rs, dtype=float)
        
        #print(batch, 'mean reward:', np.mean(rs))
        mean_reward.append(np.mean(rs))
        all_w1.append(agent.w1.copy())
        all_w2.append(agent.w2.copy())
        
        # standardize rewards (apparently helps to control gradient estimator variance)
        if np.std(rs) > 1e-9:
            rs -= np.mean(rs)
            rs /= np.std(rs)
        else:
            rs -= np.mean(rs)
        
        ps *= rs[:, None]
        
        dw1, dw2 = agent.backward_step(xs, hs, ps)
        
        upda += np.sum(dw2 != 0) + np.sum(dw1 != 0)
        noup += np.sum(dw2 == 0) + np.sum(dw1 == 0)
        
        rmsprop_dw1 = DECAY_RATE * rmsprop_dw1 + (1 - DECAY_RATE) * dw1**2
        rmsprop_dw2 = DECAY_RATE * rmsprop_dw2 + (1 - DECAY_RATE) * dw2**2
        
        # L1 regularization
        agent.w1 -= np.sign(agent.w1) * L1_FACTOR
        agent.w2 -= np.sign(agent.w2) * L1_FACTOR
        
        agent.w1 += LEARNING_RATE * dw1 / (np.sqrt(rmsprop_dw1) + 1e-5)
        agent.w2 += LEARNING_RATE * dw2 / (np.sqrt(rmsprop_dw2) + 1e-5)
    
    np.savez('ai4d.npz', w1=agent.w1, w2=agent.w2, mean_reward=mean_reward)
    plt.subplot(2, 1, 1)
    plt.plot(mean_reward)
    plt.grid()
    plt.subplot(2, 1, 2)
    plt.plot(np.reshape(all_w2, (len(all_w2), -1)), alpha=0.5)
    plt.grid()

    plt.figure()
    
    mapdata = mapdata0.copy()
    _, _, _, _, path = run_agent(n_steps * 10, pos.copy(), agent.w1, agent.w2, mapdata, deterministic=True)
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
    
    plt.show()
    
    print(datetime.now(), upda, noup)
