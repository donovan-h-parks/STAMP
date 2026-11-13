"""Run the original STAMP unit tests (legacy/STAMP_test.py) headlessly."""
import sys
import unittest

import legacy_env
legacy_env.setup()

import numpy
numpy.seterr(all='ignore')

import STAMP_test

if __name__ == '__main__':
    suite = unittest.defaultTestLoader.loadTestsFromModule(STAMP_test)
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
