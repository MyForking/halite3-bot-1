import subprocess
import json
import numpy as np
import numba as nb
import matplotlib.pyplot as plt

def get_map():
    x = subprocess.run(['../halite', '--no-replay', '--no-logs', '--results-as-json', 'true', 'true'], capture_output=True)
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
    def __init__(self, input_shape, n_hidden):
        ws = np.random.randn(n_hidden, *input_shape) / np.sqrt(np.prod(input_shape))
        w1 = np.random.randn(n_hidden, *input_shape) / np.sqrt(np.prod(input_shape))
        w2 = np.random.randn(n_hidden) / np.sqrt(n_hidden)
        self.set_weights(ws, w1, w2)
        
    def set_weights(self, ws, w1, w2):
        w1 = np.asarray(w1)
        w2 = np.asarray(w2)
        self.w1 = np.stack([ws, w1, np.rot90(w1, 1, axes=(1, 2)), np.rot90(w1, 2, axes=(1, 2)), np.rot90(w1, 3, axes=(1, 2))])
        self.w2 = w2        
    
    @nb.jit
    def forward_step(self, x):
        h = np.maximum(0, np.einsum('khij,ij->kh', self.w1, x))  # hidden layer relu activations
        logp = np.einsum('h,kh->k', self.w2, h)
        return logp, h
    
    @nb.jit
    def backward_step(self, nx, nh, ndlogp):
        """nx, nh and nlogp are stacks of collected hidden states and gradient-logprobs"""
        dw2 = np.einsum('nkh,nk->h', nh, ndlogp)
        dh = np.einsum('nk,h->nkh', ndlogp, self.w2)
        dh[nh <= 0] = 0
        dw1 = np.einsum('nkh,nij->khij', dh, nx)
        return dw1, dw2
              
    
@nb.jit
def forward_step(x, w1, w2):
    h = np.maximum(0, np.einsum('khij,ij->kh', w1, x))  # hidden layer relu activations
    logp = np.einsum('h,kh->k', w2, h)
    return logp, h
    
@nb.jit    
def run_agent(n_steps, pos, w1, w2, mapdata):
    collected = 0
    
    xs = np.empty((n_steps, 2*r+1, 2*r+1))
    hs = np.empty((n_steps, w1.shape[0], w1.shape[1]))
    ps = np.empty((n_steps, 5))
    
    for i in range(n_steps):
        pos = width + pos % width
        x = mapdata[pos[0]-r:pos[0]+r+1, pos[1]-r:pos[1]+r+1]
        logp, h = forward_step(x, w1, w2)
        p = softmax(logp)
        action = np.random.choice(5, p=p)
        y = np.zeros(5); y[action] = 1
        
        if action == 0:
            amount = np.ceil(mapdata[pos[0], pos[1]] / 4)
            mapdata[pos[0], pos[1]] -= amount
            collected += amount
        elif action == 1:
            pos[0] += 1
        elif action == 2:
            pos[1] += 1
        elif action == 3:
            pos[0] -= 1
        elif action == 4:
            pos[1] -= 1
            
        xs[i] = x
        hs[i] = h
        ps[i] = y - p
    return collected, xs, hs, ps
    
    
mapdata = get_map()    
#mapdata = (np.linspace(0, 1000, 15)[:, None] * np.ones((1, 15))).T
#mapdata = np.ones((15, 15)) * 1000

width, height = mapdata.shape
assert width == height
mapdata0 = np.tile(mapdata, [3, 3])

LEARNING_RATE = 1e-3
DECAY_RATE = 0.9

batchsize = 100
r = 2
n_steps = 50
agent = Agent((r*2+1, r*2+1), 20)

#ws = np.zeros((1, 5, 5))
#ws[0, 2, 2] = 0.1
#w1 = np.zeros((1, 5, 5))
#w1[0, 3, 2] = 0.1
#w2 = [1.0]
#agent.set_weights(ws, w1, w2)

rmsprop_dw1 = np.zeros_like(agent.w1)
rmsprop_dw2 = np.zeros_like(agent.w2)

mean_reward = []

batch = 0
while True:
    batch += 1
    xs, hs, ps, rs = [], [], [], []
    pos = np.random.randint(width, 2*width, 2)
    for _ in range(batchsize):
        collected, x, h, p = run_agent(n_steps, pos.copy(), agent.w1, agent.w2, mapdata0.copy())
        rs.extend([collected] * n_steps)
        xs.extend(x)
        hs.extend(h)
        ps.extend(p)
    
    xs = np.array(xs)
    hs = np.array(hs)
    ps = np.array(ps)
    rs = np.array(rs, dtype=float)
    
    #print(batch, 'mean reward:', np.mean(rs))
    mean_reward.append(np.mean(rs))
    if batch % 100 == 0:
        plt.plot(mean_reward)
        plt.show()
    
    # standardize rewards (apparently helps to control gradient estimator variance)
    if np.std(rs) > 1e-9:
        rs -= np.mean(rs)
        rs /= np.std(rs)
    else:
        rs -= np.mean(rs)
    
    ps *= rs[:, None]
    
    dw1, dw2 = agent.backward_step(xs, hs, ps)
    
    rmsprop_dw1 = DECAY_RATE * rmsprop_dw1 + (1 - DECAY_RATE) * dw1**2
    rmsprop_dw2 = DECAY_RATE * rmsprop_dw2 + (1 - DECAY_RATE) * dw2**2
    
    agent.w1 += LEARNING_RATE * dw1 / (np.sqrt(rmsprop_dw1) + 1e-5)
    agent.w2 += LEARNING_RATE * dw2 / (np.sqrt(rmsprop_dw2) + 1e-5)
