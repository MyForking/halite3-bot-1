import numpy as np
import scipy as sp
import scipy.optimize as spo
import matplotlib.pyplot as plt
import subprocess
import json
import numba as nb

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
    scoreMatExp = np.exp(x)
    return scoreMatExp / scoreMatExp.sum(0)
    
mapdata = get_map()    
mapdata = mapdata[:15, :15]
#mapdata = (np.linspace(0, 1000, 15)[:, None] * np.ones((1, 15))).T

width, height = mapdata.shape
assert width == height
mapdata = np.tile(mapdata, [3, 3])
plt.imshow(mapdata)

r = 2
k = r * 2 + 1

def simulate_nojit(pos, w1, w2, w3, w4, s_weights, bias, mapdata, path=False):
    bm, bs = bias
        
    pos = width + pos % width
    pos = width + pos % width
    pos = width + pos % width
    pos = width + pos % width
    pos = width + pos % width
    
    if path:
        points = []
    
    cargo = 0
    
    for _ in range(100):
        pos = width + pos % width
        window = mapdata[pos[0]-r:pos[0]+r+1, pos[1]-r:pos[1]+r+1] / 1000
        
        a = np.sum(window * w1) + bm
        b = np.sum(window * w2) + bm
        c = np.sum(window * w3) + bm
        d = np.sum(window * w4) + bm
        e = np.sum(window * s_weights) + bs
        
        p = softmax(np.array([e, a, b, c, d]))
        d = np.argmax(p)
                
        if d == 0:
            amount = np.ceil(mapdata[pos[0], pos[1]] / 4)
            mapdata[pos[0], pos[1]] -= amount
            cargo += amount
            if path:
                points.append(pos)
        elif d == 1:
            pos[0] += 1
        elif d == 2:
            pos[1] += 1
        elif d == 3:
            pos[0] -= 1
        elif d == 4:
            pos[1] -= 1
            
    if path:
        return cargo, np.array(points)
            
    return cargo

@nb.jit(nopython=True)
def simulate(pos, w1, w2, w3, w4, s_weights, bias, mapdata):
    bm, bs = bias
        
    pos = width + pos % width
    pos = width + pos % width
    pos = width + pos % width
    pos = width + pos % width
    pos = width + pos % width
    
    cargo = 0
    
    for _ in range(100):
        pos = width + pos % width
        window = mapdata[pos[0]-r:pos[0]+r+1, pos[1]-r:pos[1]+r+1] / 1000
        
        a = np.sum(window * w1) + bm
        b = np.sum(window * w2) + bm
        c = np.sum(window * w3) + bm
        d = np.sum(window * w4) + bm
        e = np.sum(window * s_weights) + bs
        
        p = softmax(np.array([e, a, b, c, d]))
        d = np.argmax(p)
                
        if d == 0:
            amount = np.ceil(mapdata[pos[0], pos[1]] / 4)
            mapdata[pos[0], pos[1]] -= amount
            cargo += amount
        elif d == 1:
            pos[0] += 1
        elif d == 2:
            pos[1] += 1
        elif d == 3:
            pos[0] -= 1
        elif d == 4:
            pos[1] -= 1
            
    return cargo
        

weights = np.random.randn(2*k, k)
bias = np.array([0, 0])

w_m = np.zeros((k, k))
w_s = np.zeros((k, k))
w_m[3, 2] = 1
w_s[2, 2] = 2
weights = np.concatenate([w_m, w_s], axis=0)

params = np.concatenate([bias.ravel(), weights.ravel()])

n_params = 2*k*k + 2
    
alpha = 0.1

weights, bias0 = visualize(params)    
#bias = bias * (1-alpha) + bb * alpha
#weights = weights * (1-alpha) + ww * alpha    
m_weights, s_weights0 = weights[:k], weights[k:]
w10 = m_weights
w20 = np.rot90(m_weights, 1)
w30 = np.rot90(m_weights, 2)
w40 = np.rot90(m_weights, 3)

    
@nb.jit
def cost(params):
    weights = params[2:].reshape(2*k, k)
    
    m_weights, s_weights = weights[:k], weights[k:]
    
    w1 = m_weights
    w2 = np.rot90(m_weights, 1)
    w3 = np.rot90(m_weights, 2)
    w4 = np.rot90(m_weights, 3)
    
    bias = params[:2]
    total = 0
    for _ in range(10):
        pos = np.random.randint(width, 2*width, 2)
        total += -simulate(pos, w1, w2, w3, w4, s_weights, bias, mapdata.copy())
    return total / 10

def visualize(params):
    weights = params[2:].reshape(2*k, k)
    a = np.max(np.abs(weights))
    bias = params[:2]
    plt.subplot(1, 2, 1)
    plt.imshow(weights[:k], vmin=-a, vmax=a, cmap='coolwarm')
    plt.subplot(1, 2, 2)
    plt.imshow(weights[k:], vmin=-a, vmax=a, cmap='coolwarm')
    print(bias)
    return weights, bias

res = spo.differential_evolution(cost, [[-100, 100]] * n_params, maxiter=1000, polish=False, disp=True)
#res = spo.minimize(cost, np.concatenate([bias, weights.ravel()]))
#print(res)
params = res.x

plt.figure()
weights, bias = visualize(params)    
#bias = bias * (1-alpha) + bb * alpha
#weights = weights * (1-alpha) + ww * alpha    
m_weights, s_weights = weights[:k], weights[k:]
w1 = m_weights
w2 = np.rot90(m_weights, 1)
w3 = np.rot90(m_weights, 2)
w4 = np.rot90(m_weights, 3)
alpha *= 0.9

plt.figure()
#simulate(pos, weights, bias, mapdata.copy(), plot=True)
pos = np.random.randint(width, 2*width, 2)
plt.imshow(mapdata)
c0, p = simulate_nojit(pos, w10, w20, w30, w40, s_weights0, bias0, mapdata.copy(), path=True)
plt.plot(*p.T[::-1], 'm.')
c, p = simulate_nojit(pos, w1, w2, w3, w4, s_weights, bias, mapdata.copy(), path=True)
plt.plot(*p.T[::-1], 'r.')
plt.title(c)
plt.xlim(width-5, 2*width+5)
plt.ylim(width-5, 2*width+5)
plt.show()
print(c0, c)
