import subprocess
import json
import numpy as np
import numba as nb
import matplotlib.pyplot as plt
import datetime
import os
from sklearn.externals.joblib import Parallel, delayed

def load_dump(filename):
    x0, h0, p0 = [], [], []
    try:
        for x, h, p in eval(open(filename).read()):
            x0.append(x)
            h0.append(h)
            p0.append(p)
        os.remove(filename)
    except FileNotFoundError:
        pass
    return np.array(x0), np.array(h0), np.array(p0)

def start_game(w1, w2, aifile, ids):        
    proc = subprocess.Popen(['../halite', '--no-timeout', '--no-replay', '--no-logs', '--width 32', '--height 32', '--results-as-json', '../target/release/my_bot ' + aifile + ' ' + ids, '../target/release/my_bot ' + aifile + ' ' + ids], stdout=subprocess.PIPE)
    return proc

def get_game(proc, ids):    
    proc.wait()
    out, err = proc.communicate()
    #print(out)
    #print(ids, err)
    res = json.loads(out)
    
    score0 = res['stats']['0']['score'] / res['map_total_halite']
    score1 = res['stats']['1']['score'] / res['map_total_halite']

    x0, h0, p0 = load_dump('netdump{}-0.txt'.format(ids))
    x1, h1, p1 = load_dump('netdump{}-1.txt'.format(ids))
    
    reward0 = np.ones(x0.shape[0]) * score0
    reward1 = np.ones(x1.shape[0]) * score1
    
    x = np.concatenate([x0, x1])
    h = np.concatenate([h0, h1])
    p = np.concatenate([p0, p1])
    reward = np.concatenate([reward0, reward1])
    
    return x, h, p, reward
              
    
def forward_step(x, w1, w2):
    h = np.maximum(0, np.einsum('hi,i->h', w1, x))  # hidden layer relu activations
    logp = np.einsum('kh,h->k', w2, h)
    return logp, h

def backward_step(nx, nh, ndlogp, w2):
    """nx, nh and nlogp are stacks of collected hidden states and gradient-logprobs"""
    dw2 = np.einsum('nh,nk->kh', nh, ndlogp)
    dh = np.einsum('nk,kh->nh', ndlogp, w2)
    dh[nh <= 0] = 0
    dw1 = np.einsum('nh,ni->hi', dh, nx)
    return dw1, dw2   

LEARNING_RATE = 1e-2
DECAY_RATE = 0.9
L1_FACTOR = 1e-3

parsize = 5
batchsize = 1
r = 2

n_input = (r*2+1)**2
n_hidden = 15
n_output = 5

w1 = np.random.randn(n_hidden, n_input) / np.sqrt(np.prod(n_input))
w2 = np.random.randn(5, n_hidden) / np.sqrt(n_hidden)

#ws = np.zeros((1, 5, 5))
#ws[0, 2, 2] = 0.1
#w1 = np.zeros((1, 5, 5))
#w1[0, 3, 2] = 0.1
#w2 = [1.0]
#agent.set_weights(ws, w1, w2)

rmsprop_dw1 = np.zeros_like(w1)
rmsprop_dw2 = np.zeros_like(w2)

mean_reward = []
all_w1 = []
all_w2 = []

upda = 0
noup = 0
nextupdate = datetime.datetime.now() + datetime.timedelta(seconds=10)
while True:    
    aifile = "net.txt"
    with open(aifile, 'w') as f:
        print('n_input:', w1.shape[1], file=f)
        print('n_hidden:', w1.shape[0], file=f)
        print('n_output:', w2.shape[0], file=f)
        print('layer1_weights:', ', '.join('{}'.format(w) for w in w1.ravel()), file=f)
        print('layer2_weights:', ', '.join('{}'.format(w) for w in w2.ravel()), file=f)
    
    xs, hs, ps, rs = [], [], [], []
    for _ in range(batchsize):
        procs = [(start_game(w1, w2, aifile, str(i)), str(i)) for i in range(parsize)]
        for proc in procs:
            x, h, p, r = get_game(*proc)
            xs.append(x)
            hs.append(h)
            ps.append(p)
            rs.append(r)
    xs = np.concatenate(xs)
    hs = np.concatenate(hs)
    ps = np.concatenate(ps)
    rs = np.concatenate(rs)
    
    #print(batch, 'mean reward:', np.mean(rs))
    mean_reward.append(np.mean(rs))
    all_w1.append(w1.copy())
    all_w2.append(w2.copy())
    
    # standardize rewards (apparently helps to control gradient estimator variance)
    if np.std(rs) > 1e-9:
        rs -= np.mean(rs)
        rs /= np.std(rs)
    else:
        rs -= np.mean(rs)
    
    ps *= rs[:, None]
    
    dw1, dw2 = backward_step(xs, hs, ps, w2)
    
    upda += np.sum(dw2 != 0) + np.sum(dw1 != 0)
    noup += np.sum(dw2 == 0) + np.sum(dw1 == 0)
    
    rmsprop_dw1 = DECAY_RATE * rmsprop_dw1 + (1 - DECAY_RATE) * dw1**2
    rmsprop_dw2 = DECAY_RATE * rmsprop_dw2 + (1 - DECAY_RATE) * dw2**2
    
    # L1 regularization
    #w1 -= np.sign(w1) * L1_FACTOR
    #w2 -= np.sign(w2) * L1_FACTOR
    
    w1 += LEARNING_RATE * dw1 / (np.sqrt(rmsprop_dw1) + 1e-5)
    w2 += LEARNING_RATE * dw2 / (np.sqrt(rmsprop_dw2) + 1e-5)
    
    print(datetime.datetime.now(), mean_reward[-1])
    
    if datetime.datetime.now() > nextupdate:
        np.savez('ai3a.npz', w1=w1, w2=w2, mean_reward=mean_reward)
        plt.subplot(2, 1, 1)
        plt.plot(mean_reward)
        plt.grid()
        plt.subplot(2, 1, 2)
        plt.plot(np.reshape(all_w2, (len(all_w2), -1)), alpha=0.5)
        plt.grid()
        plt.show()
        nextupdate = datetime.datetime.now() + datetime.timedelta(minutes=1)
    
