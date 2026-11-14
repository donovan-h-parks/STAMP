# Sphinx configuration for the STAMP documentation.
#
# Build with:  sphinx-build -b html docs/site docs/site/_build/html
# (pip install -r docs/site/requirements.txt)

import pathlib
import tomllib

# The version is set once, in the workspace's Cargo.toml.
_root = pathlib.Path(__file__).resolve().parents[2]
with open(_root / "Cargo.toml", "rb") as f:
    release = tomllib.load(f)["workspace"]["package"]["version"]
version = release

project = "STAMP"
author = "Donovan Parks and Rob Beiko"
copyright = "2010-2026, Donovan Parks and Rob Beiko"

extensions = []
exclude_patterns = ["_build", "requirements.txt"]

html_theme = "sphinx_rtd_theme"
html_logo = "_static/stamp_icon.png"
html_static_path = ["_static"]
html_css_files = ["stamp.css"]
html_title = f"STAMP {release} documentation"
html_theme_options = {
    "navigation_depth": 3,
    "collapse_navigation": False,
}
# The "Edit on GitHub" link of each page.
html_context = {
    "display_github": True,
    "github_user": "donovan-h-parks",
    "github_repo": "STAMP",
    "github_version": "rust-port",
    "conf_py_path": "/docs/site/",
}
