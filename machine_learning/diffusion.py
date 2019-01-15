import numpy as np
import matplotlib.pyplot as plt
import scipy as sp
from scipy import integrate as spi
from scipy import signal


DIFFUSION_COEFFICIENT = 10
EVAPORATION_RATE = 1
DECAY_RATE = 0.25

W = H = 32
    
    
LAPLACIAN = 0 - 4 * np.eye(W*H)
i, j = np.unravel_index(np.arange(W*H), (H, W))
LAPLACIAN[np.arange(W*H), np.ravel_multi_index((i+1, j), (H, W), mode='wrap')] = 1
LAPLACIAN[np.arange(W*H), np.ravel_multi_index((i-1, j), (H, W), mode='wrap')] = 1
LAPLACIAN[np.arange(W*H), np.ravel_multi_index((i, j+1), (H, W), mode='wrap')] = 1
LAPLACIAN[np.arange(W*H), np.ravel_multi_index((i, j-1), (H, W), mode='wrap')] = 1


def steady_state(sources):
    """assuming linear sources (no use of maximum)"""
    np.linalg.inv((DECAY_RATE + EVAPORATION_RATE) * np.eye(W*H) - DIFFUSION_COEFFICIENT * LAPLACIAN) * EVAPORATION_RATE
    -np.linalg.inv(DIFFUSION_COEFFICIENT * LAPLACIAN - (DECAY_RATE + EVAPORATION_RATE) * np.eye(W*H)) @ sources.ravel() * EVAPORATION_RATE
    np.linalg.solve((DECAY_RATE + EVAPORATION_RATE) * np.eye(W*H) - DIFFUSION_COEFFICIENT * LAPLACIAN, sources.ravel() * EVAPORATION_RATE)


def derivatives(t, concentration):
    # diffusion
    #dcdt = DIFFUSION_COEFFICIENT * LAPLACIAN @ concentration
    dcdt = DIFFUSION_COEFFICIENT * signal.convolve2d(concentration, [[0, 1, 0], [1, -4, 1], [0, 1, 0]], mode='same', boundary='wrap')
    
    # sources
    dcdt += np.maximum(0, sources - concentration) * EVAPORATION_RATE
    #dcdt += (sources.ravel() - concentration) * EVAPORATION_RATE
    
    # sinks
    #dcdt[330] += np.minimum(0, 100 - concentration[330]) * 5
    
    # decay
    dcdt -= concentration * DECAY_RATE
    #dcdt -= np.maximum(0, np.sum(concentration) - np.sum(sources)) * DECAY_RATE
    #dcdt -= (concentration > 1) * DECAY_RATE
    
    return dcdt



sources = np.random.rand(H, W) * 100
#sources[H//2, W//2] = 1000

#phi0 = np.zeros((H, W)) + 100
phi0 = np.random.rand(H, W) * 000.0
#phi0[H//2, W//2] = 1

#res = spi.solve_ivp(derivatives, (0, 20), phi0.ravel())


phi = phi0.copy()
phis, ts = [], []
t = 0
dt = 2e-2

for i in np.arange(0, 1, dt):    
    dphi = derivatives(t, phi)
    #dphi1 = derivatives(t+dt, phi + dphi*dt)
    #dphi = (dphi + dphi1) * 0.5
    phi += dphi * dt    
    t += dt
    phis.append(phi.copy())
    ts.append(t)
    
#sources[16:] *= 0.1
#sources[H//4, W//4] = 1000
sources[H//2, W//2] = 1000
#sources[H//2 + 5, W//2] = 1000
#sources[H//2 + 5, W//2 + 5] = 1000
    
for i in np.arange(1, 10, dt):      
    dphi = derivatives(t, phi)
    #dphi1 = derivatives(t+dt, phi + dphi*dt)
    #dphi = (dphi + dphi1) * 0.5
    phi += dphi * dt
    
    t += dt
    phis.append(phi.copy())
    ts.append(t)
    
phis = np.array(phis)

plt.plot(ts, phis.reshape(-1, H*W))

plt.figure()
plt.plot(np.mean(sources, axis=1)); plt.plot(np.mean(phi, axis=1))