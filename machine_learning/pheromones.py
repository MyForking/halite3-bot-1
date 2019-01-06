import numpy as np
import scipy as sp


EVAPORATION_RATE = 0.9
DIFFUSION_RATE = 0.2;

W = 16;
H = 16;


front = np.zeros((H, W))

for _ in range(100):
    front[8, 8] += 1000
    back = front * EVAPORATION_RATE
    for i in range(H):
        for j in range(W):            
            dy = (front[(i + 1) % H, j] - front[i, j]) * DIFFUSION_RATE;
            dx = (front[i, (j + 1) % W] - front[i, j]) * DIFFUSION_RATE;
    
            back[(i + 1) % H, j] -= dy;
            back[i, (j + 1) % W] -= dx;
    
            back[i, j] +=  dx + dy;
            
    front, back = back, front

plt.imshow(front, vmin=0, vmax=1000); plt.colorbar()
