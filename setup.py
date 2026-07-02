#!/usr/bin/env python3
# Metadata and dependencies now live in pyproject.toml.
# This shim remains only so that legacy `python setup.py ...` invocations
# and some packaging tools continue to work. distutils (removed in Python
# 3.12) and pyqi (archived, Python-2-only) are no longer used.
from setuptools import setup

if __name__ == "__main__":
    setup()
