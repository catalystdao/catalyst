# Set hypothesis custom test parameters
from hypothesis import settings
settings.register_profile('default', max_examples=int(1e3)) # 1e5 takes approx 45s per test
settings.load_profile('default')

# Used for imports from fixed_point_math.py and integer.py
import sys
import os
sys.path.insert(1, os.path.join(sys.path[0], '../'))