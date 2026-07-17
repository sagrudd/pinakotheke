"""Sphinx configuration for the x-img planning documentation."""

# SPDX-License-Identifier: MPL-2.0
project = "Pinakotheke"
copyright = "2026, x-img maintainers"
author = "x-img maintainers"
release = "1.2.1"

extensions = ["myst_parser"]
templates_path = ["_templates"]
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]
html_theme = "alabaster"
html_title = "Pinakotheke documentation"
myst_heading_anchors = 3
