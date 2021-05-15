#!/usr/bin/env python3

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.patches as patches
import cbor2
import sys

fig = plt.figure()
ax1 = fig.add_subplot(1, 1, 1)

for filename in sys.argv[1:]:
	state = cbor2.load(open(filename, 'rb'))
	for patch in state['primitive'][:]:
	    i0 = patch['rect'][0]['start']
	    j0 = patch['rect'][1]['start']
	    i1 = patch['rect'][0]['end']
	    j1 = patch['rect'][1]['end']
	    x, y = np.meshgrid(range(i0, i1 + 1), range(j0, j1 + 1))
	    data = np.array(patch['data']).reshape([i1 - i0, j1 - j0, patch['num_fields']])
	    cm = ax1.pcolormesh(x, y, data[:,:,0].T, vmin=0.0, vmax=1.0)
	    box = patches.Rectangle((i0, j0), i1 - i0, j1 - j0, linewidth=0.5, edgecolor='k', fill=False)
	    ax1.add_patch(box)

ax1.set_aspect('equal')
fig.colorbar(cm)
plt.show()
