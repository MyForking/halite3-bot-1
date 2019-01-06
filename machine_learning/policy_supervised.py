import numpy as np
import scipy.ndimage as ndimage
import random
import matplotlib.pyplot as plt

from sklearn.discriminant_analysis import LinearDiscriminantAnalysis as LDA
from sklearn.tree import DecisionTreeClassifier
from sklearn.naive_bayes import GaussianNB
from sklearn.neural_network import MLPClassifier

MOVE_FACTOR = 10
GAIN_FACTOR = 4

W = 32
H = 32

def generate_map(w, h):
    data = np.zeros((h, w))
    
    for s in range(1, 5):    
        data += ndimage.gaussian_filter(np.random.rand(h, w), sigma=(s, s), order=0, mode='wrap') * s**2
        
    data -= np.min(data)
    data *= 1000 / np.max(data)
    
    return data
    
    
def move(pos, d):
    if d == 'n':
        return (pos[0] % H, (pos[1] - 1) % W)
    if d == 's':
        return (pos[0] % H, (pos[1] + 1) % W)
    if d == 'e':
        return ((pos[0] - 1) % H, pos[1] % W)
    if d == 'w':
        return ((pos[0] + 1) % H, pos[1] % W)
    return pos
    
    
def greedy_policy(data, pos):
    move_cost = np.floor(data[pos] / MOVE_FACTOR)
    stay_gain = np.ceil(data[pos] / GAIN_FACTOR)
    
    best = [stay_gain, 'x']
    
    order = list('news')
    np.random.shuffle(order)
    
    for d in order:
        p = move(pos, d)
        move_value = np.ceil(data[p] / GAIN_FACTOR) - move_cost
        if move_value > best[0]:
            best = [move_value, d]
    
    return best[1]


class SklearnPolicy:
    def __init__(self, cla, patch_size):
        self.cla = cla
        self.patch_size = patch_size
        
    def __call__(self, data, pos):
        x = np.empty((self.patch_size, self.patch_size))
        for i, a in enumerate(range(pos[0]-self.patch_size//2, pos[0]+self.patch_size//2+1)):
            for j, b in enumerate(range(pos[1]-self.patch_size//2, pos[1]+self.patch_size//2+1)):
                x[i, j] = data[a % H, b % W]
        return self.cla.predict(x.reshape(1, -1))


def simulate_step(data, pos, d):
    if d == 'x':
        gain = np.ceil(data[pos] / GAIN_FACTOR)
        data[pos] -= gain
        return gain, pos
    else:
        pos = move(pos, d)
        return -np.floor(data[pos] / MOVE_FACTOR), pos
    

def eval_policy(data0, start, policy, plot=False):
    data = data0.copy()        
    
    path = [start]
    pos = start
    total = 0
    
    for i in range(100):
        d = policy(data, pos)
        gain, pos = simulate_step(data, pos, d)        
        total += gain
        path.append(pos)
        
    if plot:
        plt.figure()
        plt.subplot(1, 2, 1)
        plt.imshow(data0, vmin=0, vmax=1000)
        plt.plot(*np.transpose(path)[::-1], 'r')
        plt.subplot(1, 2, 2)
        plt.imshow(data, vmin=0, vmax=1000)
        plt.suptitle(total)
            
    return total, path


def gen_data(data0, patch_size, policy, n_steps, n_batches):
    X = []
    Y = []
    for batch in range(n_batches):
        data = data0.copy()
        pos = (np.random.randint(0, H), np.random.randint(0, W))
        for step in range(n_steps):
            x = np.empty((patch_size, patch_size))
            for i, a in enumerate(range(pos[0]-patch_size//2, pos[0]+patch_size//2+1)):
                for j, b in enumerate(range(pos[1]-patch_size//2, pos[1]+patch_size//2+1)):
                    x[i, j] = data[a % H, b % W]
            d = policy(data, pos)
            X.append(x)
            Y.append(d)
            gain, pos = simulate_step(data, pos, d)
    return np.array(X), np.array(Y)
    
    
data = generate_map(W, H)
print(eval_policy(data, (10, 10), greedy_policy, plot=True))


x, y = gen_data(data, 5, greedy_policy, 10000, 10)
x = x.reshape(x.shape[0], -1)

x_test, y_test = gen_data(data, 5, greedy_policy, 10000, 1)
x_test = x_test.reshape(x_test.shape[0], -1)

lda = LDA().fit(x, y)
print('lda score:', lda.score(x_test, y_test), eval_policy(data, (10, 10), SklearnPolicy(lda, 5), plot=True)[0])

tree = DecisionTreeClassifier(max_depth=10).fit(x, y)
print('tree score:', tree.score(x_test, y_test), eval_policy(data, (10, 10), SklearnPolicy(tree, 5), plot=True)[0])

gnb = GaussianNB().fit(x, y)
print('naive bayes score:', gnb.score(x_test, y_test), eval_policy(data, (10, 10), SklearnPolicy(gnb, 5), plot=True)[0])

mlp = MLPClassifier().fit(x, y)
print('mlp score:', mlp.score(x_test, y_test), eval_policy(data, (10, 10), SklearnPolicy(mlp, 5), plot=True)[0])