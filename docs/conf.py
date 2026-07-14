"""Sphinx configuration for the x-img planning documentation."""

project = "x-img"
copyright = "2026, x-img maintainers"
author = "x-img maintainers"
release = "0.1.0"

extensions = ["myst_parser"]
templates_path = ["_templates"]
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]
html_theme = "alabaster"
html_title = "x-img documentation"
myst_heading_anchors = 3
