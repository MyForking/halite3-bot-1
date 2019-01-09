import numpy as np
import matplotlib.pyplot as plt


def objective(x):
    return np.exp(-(x-2)**2) + np.exp(-(x+2)**2)


mu = 0
sigma2 = 10
t = 0
maxits = 100
n = 100
ne = 10

epsilon = 1e-12

plt.figure()
while t < maxits and sigma2 > epsilon:
    x = np.random.randn(n) * np.sqrt(sigma2) + mu    
    s = objective(x)
    plt.plot(x, s + t, '.')
    x = x[np.argsort(s)]    
    mu = np.mean(x[-ne:])
    sigma2 = np.var(x[-ne:])
    t += 1

plt.gca().axvline(mu, color='r')


a = -6
b = 6
t = 0
maxits = 100
n = 100
ne = 10

epsilon = 1e-6

plt.figure()
while t < maxits and b - a > epsilon:
    x = np.random.rand(n) * (b - a) + a
    s = objective(x)
    plt.plot(x, s + t, '.')
    x = x[np.argsort(s)]
    a = np.min(x[-ne:])
    b = np.max(x[-ne:])
    t += 1

plt.gca().axvline(a, color='r')
plt.gca().axvline(b, color='r')