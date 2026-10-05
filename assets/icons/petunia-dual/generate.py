#!/usr/bin/env python3
"""Generate the first-party Petunia icon family; no external icon assets.

Run from any directory: python3 assets/icons/petunia-dual/generate.py
The geometry below is the editable source. Output SVGs and the gallery are committed.
"""

from __future__ import annotations

import html
import json
from pathlib import Path
import re
import shutil
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
SOURCE = Path(__file__).resolve().parent
ICONS: dict[str, tuple[str, str, str, str, str]] = {}


def p(d: str, **attrs: str) -> str:
    return f'<path d="{d}"' + ''.join(f' {k.replace("_", "-")}="{v}"' for k, v in attrs.items()) + '/>'


def r(x: float, y: float, w: float, h: float, rx: float = 0, **attrs: str) -> str:
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}"' + ''.join(f' {k.replace("_", "-")}="{v}"' for k, v in attrs.items()) + '/>'


def c(x: float, y: float, radius: float, **attrs: str) -> str:
    return f'<circle cx="{x}" cy="{y}" r="{radius}"' + ''.join(f' {k.replace("_", "-")}="{v}"' for k, v in attrs.items()) + '/>'


def solid(*bits: str) -> str:
    return '<g fill="currentColor" stroke="none">' + ''.join(bits) + '</g>'


def ink(*bits: str) -> str:
    return '<g fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">' + ''.join(bits) + '</g>'


def detail(*bits: str) -> str:
    return '<g fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">' + ''.join(bits) + '</g>'


def add(key: str, group: str, label: str, outline: str, filled: str, status: str = 'current') -> None:
    assert re.fullmatch(r'[a-z][a-z0-9_]*', key) and key not in ICONS, key
    assert outline and filled, key
    ICONS[key] = group, label, outline, filled, status


def pair(key: str, group: str, label: str, path: str, fill_path: str | None = None, status: str = 'current') -> None:
    add(key, group, label, ink(p(path)), solid(p(fill_path or path)), status)


def badge(base: str, mark_o: str, mark_f: str, key: str, group: str, label: str, status: str = 'current') -> None:
    """A base silhouette occupies 17 px; an independently legible mark occupies the corner."""
    _, _, o, f, _ = ICONS[base]
    add(key, group, label,
        f'<g transform="translate(1 1) scale(.78)">{o}</g>' + c(18, 18, 5, fill='#252735', stroke='currentColor', stroke_width='1.75') + ink(mark_o),
        f'<g transform="translate(1 1) scale(.78)">{f}</g>' + c(18, 18, 5, fill='#252735') + solid(mark_f), status)


G = 'Interface'
add('search', G, 'Search', ink(c(10.5, 10.5, 6.5), p('15.5 15.5 21 21')), solid(p('M10.5 2a8.5 8.5 0 0 1 6.54 13.92L22 20.9 20.9 22l-4.98-4.96A8.5 8.5 0 1 1 10.5 2zm0 2.2a6.3 6.3 0 1 0 0 12.6 6.3 6.3 0 0 0 0-12.6z', fill_rule='evenodd')))
add('settings', G, 'Settings', ink(c(12, 12, 3), p('M10 2h4l.5 2.3 1.7.7 2-.9 2.8 2.8-.9 2 .7 1.7L23 11v3l-2.2.5-.7 1.7.9 2-2.8 2.8-2-.9-1.7.7L14 23h-4l-.5-2.2-1.7-.7-2 .9L3 18.2l.9-2-.7-1.7L1 14v-3l2.2-.5.7-1.7-.9-2L5.8 4l2 .9 1.7-.7z')), solid(p('M9.2 2h5.6l.47 2.2 1.26.52 1.88-1.2 3.96 3.96-1.2 1.88.52 1.26L24 11v5.6l-2.31.47-.52 1.26 1.2 1.88-3.96 3.96-1.88-1.2-1.26.52L14.8 24H9.2l-.47-2.31-1.26-.52-1.88 1.2-3.96-3.96 1.2-1.88-.52-1.26L0 14.8V9.2l2.31-.47.52-1.26-1.2-1.88 3.96-3.96 1.88 1.2 1.26-.52zM12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z', fill_rule='evenodd')))
add('folder', G, 'Folder', ink(p('M2 6a2 2 0 0 1 2-2h5l2 2h9a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z')), solid(p('M2 6a2 2 0 0 1 2-2h5l2 2h9a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z')))
add('file', G, 'File', ink(p('M5 2h9l5 5v15H5z'), p('M14 2v5h5')), solid(p('M5 2h9l5 5v15H5z')))
add('new_file', G, 'New Project', ink(p('M5 2h9l5 5v15H5z'), p('M14 2v5h5M9 15h6m-3-3v6')), solid(p('M5 2h9l5 5v15H5z')) + ink(p('M9 15h6m-3-3v6', stroke='#252735')))
add('save', G, 'Save Project', ink(p('M3 3h15l3 3v15H3z'), p('M7 3v6h10V3M7 21v-8h10v8')), solid(p('M3 3h15l3 3v15H3z')) + r(7, 4, 10, 5, fill='#252735', stroke='none') + r(7, 13, 10, 8, fill='#252735', stroke='none'))
badge('save', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'save_as', G, 'Save As')
badge('folder', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'open_project', G, 'Open Project')
add('import', G, 'Import', ink(p('M4 19v2h16v-2M12 3v12m-4-4 4 4 4-4M5 6h5m4 0h5')), solid(p('M11 2h2v10l2.5-2.5 1.6 1.6L12 16.2 6.9 11.1l1.6-1.6L11 12zM3 18h2v3h14v-3h2v5H3z')))
add('export', G, 'Export', ink(p('M4 19v2h16v-2M12 16V4m-4 4 4-4 4 4M5 15h5m4 0h5')), solid(p('M11 15h2V5l2.5 2.5 1.6-1.6L12 .8 6.9 5.9l1.6 1.6L11 5zM3 18h2v3h14v-3h2v5H3z')))
add('undo', G, 'Undo', ink(p('M9 6 4 11l5 5M4 11h10a6 6 0 0 1 0 12')), solid(p('M9 4 2 11l7 7v-5h5a4 4 0 0 1 4 4v3h3v-3a7 7 0 0 0-7-7H9z')))
add('redo', G, 'Redo', ink(p('m15 6 5 5-5 5m5-5H10a6 6 0 0 0-6 6')), solid(p('m15 4 7 7-7 7v-5h-5a4 4 0 0 0-4 4v3H3v-3a7 7 0 0 1 7-7h5z')))
add('duplicate', G, 'Duplicate', ink(r(3, 3, 12, 12, 1), p('M9 18v3h12V9h-3')), solid(r(3, 3, 12, 12, 1), p('M17 9h4v12H9v-4h2v2h8v-8h-2z')))
add('delete', G, 'Delete', ink(p('M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7')), solid(p('M9 2h6l1 3h5v2H3V5h5zM5 9h14l-1 13H6z')))
add('plus', G, 'Add', ink(p('M12 4v16M4 12h16')), solid(p('M10.5 3h3v7.5H21v3h-7.5V21h-3v-7.5H3v-3h7.5z')))
add('minus', G, 'Remove', ink(p('M4 12h16')), solid(r(3, 10.5, 18, 3, 1)))
add('close', G, 'Close', ink(p('M5 5l14 14M19 5 5 19')), solid(p('M6 4 4 6l6 6-6 6 2 2 6-6 6 6 2-2-6-6 6-6-2-2-6 6z')))
add('eye', G, 'Visible', ink(p('M2 12s3.7-6 10-6 10 6 10 6-3.7 6-10 6S2 12 2 12z'), c(12, 12, 2.5)), solid(p('M12 4C5.5 4 2 12 2 12s3.5 8 10 8 10-8 10-8-3.5-8-10-8zm0 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8z', fill_rule='evenodd')))
add('eye_hidden', G, 'Hidden', ink(p('M3 3 21 21M4.5 7.5C2.7 9.3 2 12 2 12s3.7 6 10 6c2 0 3.7-.6 5.1-1.4M9 6.5a11 11 0 0 1 3-.5c6.3 0 10 6 10 6s-.6 1-1.7 2.2')), solid(p('M3.4 2 2 3.4l4 4A15 15 0 0 0 2 12s3.7 8 10 8a10 10 0 0 0 5-1.3l3.6 3.3 1.4-1.4zM12 4a11 11 0 0 0-3.2.5l3 3a4.5 4.5 0 0 1 4.7 4.7l3.1 3C21.3 13.4 22 12 22 12S18.5 4 12 4z')))
add('lock', G, 'Lock', ink(r(4, 10, 16, 12, 2), p('M7 10V7a5 5 0 0 1 10 0v3'), c(12, 16, 1)), solid(p('M7 10V7a5 5 0 0 1 10 0v3h1a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2zm2 0h6V7a3 3 0 0 0-6 0z', fill_rule='evenodd')))
add('unlock', G, 'Unlock', ink(r(4, 10, 16, 12, 2), p('M8 10V7a5 5 0 0 1 9-3'), c(12, 16, 1)), solid(p('M8 10V7a5 5 0 0 1 9-3l-1.7 1.3A3 3 0 0 0 10 7v3h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z')))
add('filter', G, 'Filter', ink(p('M3 4h18l-7 8v6l-4 3v-9z')), solid(p('M2 3h20l-7 9v6l-6 4V12z')))
add('pin', G, 'Pin', ink(p('M9 3h6l-1 6 3 3v2H7v-2l3-3zM12 14v8')), solid(p('M8 2h8l-1 7 3 3v3h-5v8h-2v-8H6v-3l3-3z')))
add('more_vert', G, 'More Options', ink(c(12, 5, 1), c(12, 12, 1), c(12, 19, 1)), solid(c(12, 5, 2), c(12, 12, 2), c(12, 19, 2)))
add('warning', G, 'Warning', ink(p('M12 2 22 21H2zM12 9v5m0 3v.5')), solid(p('M12 2 24 22H0z')) + ink(p('M12 9v5m0 3v.5', stroke='#252735')))
add('help', G, 'Help', ink(c(12, 12, 10), p('M9 9a3 3 0 1 1 5 2c-1.5 1-2 1.6-2 3m0 4v.2')), solid(p('M12 1a11 11 0 1 0 0 22 11 11 0 0 0 0-22zm0 17a1.4 1.4 0 1 1 0 2.8 1.4 1.4 0 0 1 0-2.8zm-2.1-9.9a3.2 3.2 0 0 1 5.3 3.6L13 14v2h-2v-2c0-2.3 2.4-3.2 2.4-4.6 0-1.3-1.6-2-2.5-.1z', fill_rule='evenodd')))
add('refresh', G, 'Refresh', ink(p('M20 8a8 8 0 0 0-14-2L4 9m0-5v5h5M4 16a8 8 0 0 0 14 2l2-3m0 5v-5h-5')), solid(p('M12 2a10 10 0 0 1 9 6h-3a7 7 0 0 0-12-1V4H3v7h7V8H8a7 7 0 0 1 12 7h3a10 10 0 0 1-19 1h3a7 7 0 0 0 11 1v3h3v-7h-7v3h2a7 7 0 0 1-12-7H1A11 11 0 0 1 12 2z')))
add('play', G, 'Play', ink(p('M7 4 20 12 7 20z')), solid(p('M6 2 22 12 6 22z')))
add('pause', G, 'Pause', ink(r(5, 4, 5, 16, 1), r(14, 4, 5, 16, 1)), solid(r(5, 3, 5, 18, 1), r(14, 3, 5, 18, 1)))
add('stop', G, 'Stop', ink(r(4, 4, 16, 16, 2)), solid(r(4, 4, 16, 16, 2)))
add('chevron_left', G, 'Previous', ink(p('m15 4-8 8 8 8')), solid(p('m15 3 2 2-7 7 7 7-2 2-9-9z')))
add('chevron_right', G, 'Next', ink(p('m9 4 8 8-8 8')), solid(p('m9 3-2 2 7 7-7 7 2 2 9-9z')))
add('chevron_up', G, 'Up', ink(p('m4 15 8-8 8 8')), solid(p('m3 15 2 2 7-7 7 7 2-2-9-9z')))
add('chevron_down', G, 'Down', ink(p('m4 9 8 8 8-8')), solid(p('m3 9 2-2 7 7 7-7 2 2-9 9z')))
add('grid_view', G, 'Grid View', ink(r(3, 3, 7, 7, 1), r(14, 3, 7, 7, 1), r(3, 14, 7, 7, 1), r(14, 14, 7, 7, 1)), solid(r(3, 3, 7, 7, 1), r(14, 3, 7, 7, 1), r(3, 14, 7, 7, 1), r(14, 14, 7, 7, 1)))
add('list_view', G, 'List View', ink(p('M9 5h12M9 12h12M9 19h12'), c(4, 5, 1), c(4, 12, 1), c(4, 19, 1)), solid(r(9, 4, 12, 2), r(9, 11, 12, 2), r(9, 18, 12, 2), c(4, 5, 2), c(4, 12, 2), c(4, 19, 2)))

# Structural geometry: every technical symbol shares the same axonometric slant.
CUBE = 'M12 2 21 7v10l-9 5-9-5V7z'
CUBE_EDGE = 'M3 7l9 5 9-5M12 12v10M3 7l9-5 9 5'
FACE = 'M4 7 19 5 21 18 6 20z'
PLANE = 'M2 15 12 9 22 15l-10 6z'
SPHERE = 'M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20z'

H = 'Shape-first'
add('reference_image', H, 'Reference Image', ink(r(2, 3, 20, 18, 2), c(8, 9, 1.5), p('M4 18l5-5 3 3 4-5 4 6')), solid(r(2, 3, 20, 18, 2)) + ink(c(8, 9, 1.5, stroke='#252735'), p('M4 18l5-5 3 3 4-5 4 6', stroke='#252735')))
add('draw_profile', H, 'Draw Profile', ink(p('M3 19 8 7l7 5 6-8M3 19h18', stroke_dasharray='2 2'), p('M3 19Q8 7 15 12T21 4'), c(3, 19, 1), c(8, 7, 1), c(15, 12, 1), c(21, 4, 1)), solid(p('M3 19Q8 7 15 12T21 4l-2-1Q15 9 8 5T1 18z')) + solid(c(3, 19, 2), c(8, 7, 2), c(15, 12, 2), c(21, 4, 2)))
add('trace_silhouette', H, 'Trace Silhouette', ink(p('M3 20c2-11 5-16 8-11s5 0 10-6M3 20h18', stroke_dasharray='3 2'), c(3, 20, 1), c(21, 3, 1)), solid(p('M2 21C4 8 8 1 12 8c2 4 4 0 8-6l2 2c-5 8-9 12-12 6-2-4-4 4-5 11z')))
add('profile_closed', H, 'Closed Profile', ink(p('M4 17Q7 2 14 6T20 16L4 17z'), c(4, 17, 1), c(14, 6, 1), c(20, 16, 1)), solid(p('M3 18Q7 1 14 5T21 17z')))
add('parts', H, 'Parts', ink(p('M4 4h8v8H4zM12 12h8v8h-8zM12 4h8v8h-8zM4 12h8v8H4z')), solid(r(3, 3, 9, 9, 1), r(13, 3, 8, 8, 1), r(3, 13, 8, 8, 1), r(12, 12, 9, 9, 1)))
add('shape_layer', H, 'Shape Layer', ink(p('M3 8 12 3l9 5-9 5zM3 13l9 5 9-5M3 18l9 5 9-5')), solid(p('M2 8 12 2l10 6-10 6zM2 13l10 6 10-6v3l-10 6L2 16z')))
add('generate_volume', H, 'Generate Volume', ink(p('M3 17V8l9-5 9 5v9l-9 5zM3 8l9 5 9-5M12 13v9'), p('M12 2v8m-3-3 3 3 3-3')), solid(p(CUBE)) + detail(p('M3 7l9 5 9-5M12 12v10', stroke='#252735')))
add('thickness', H, 'Thickness', ink(p('M3 6 17 3l4 3-14 3zM3 13l4 3 14-3M3 18l4 3 14-3'), p('M17 9v4m-2-2 2 2 2-2')), solid(p('M3 5 17 2l5 5-15 4zM2 13l5 3 15-4v3L7 20l-5-3z')))
add('symmetry_axis', H, 'Symmetry Axis', ink(p('M12 2v20', stroke_dasharray='2 2'), p('M10 5 3 9l7 4zM14 5l7 4-7 4zM10 15 3 19l7 3zM14 15l7 4-7 3z')), solid(p('M11 2h2v20h-2zM9 5 2 9l7 5zM15 5l7 4-7 5zM9 14l-7 5 7 3zM15 14l7 5-7 3z')))

P = 'Primitives'
add('cube', P, 'Cube', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7')), solid(p(CUBE)) + detail(p('M3 7l9 5 9-5M12 12v10', stroke='#252735')))
add('plane', P, 'Plane', ink(p(PLANE), p('M2 15v2l10 5 10-5v-2')), solid(p('M2 15 12 9l10 6v2l-10 6L2 17z')))
add('sphere', P, 'Sphere', ink(c(12, 12, 9), p('M3 12h18M12 3c-6 4-6 14 0 18M12 3c6 4 6 14 0 18')), solid(p(SPHERE)) + ink(p('M4 12h16M12 3c-5 5-5 13 0 18', stroke='#252735')))
add('icosphere', P, 'Icosphere', ink(p('M12 2 21 8l-3 11-11 2L2 10zM12 2l-2 9 8 8M2 10l8 1-3 10M10 11l11-3')), solid(p('M12 2 21 8l-3 11-11 2L2 10z')) + detail(p('M12 2l-2 9 8 8M2 10l8 1-3 10M10 11l11-3', stroke='#252735')))
add('cylinder', P, 'Cylinder', ink(p('M4 7a8 4 0 0 1 16 0v10a8 4 0 0 1-16 0zM4 7a8 4 0 0 0 16 0')), solid(p('M4 7a8 4 0 0 1 16 0v10a8 4 0 0 1-16 0z')) + detail(p('M4 7a8 4 0 0 0 16 0', stroke='#252735')))
add('cone', P, 'Cone', ink(p('M12 2 3 18a9 3 0 0 0 18 0zM3 18a9 3 0 0 1 18 0')), solid(p('M12 2 3 18a9 3 0 0 0 18 0z')))
add('torus', P, 'Torus', ink(p('M12 5c-6 0-10 3-10 7s4 7 10 7 10-3 10-7-4-7-10-7z'), p('M12 9c-3 0-5 1.2-5 3s2 3 5 3 5-1.2 5-3-2-3-5-3z')), solid(p('M12 4C5.5 4 1 7.3 1 12s4.5 8 11 8 11-3.3 11-8-4.5-8-11-8zm0 5c-3 0-5 1.2-5 3s2 3 5 3 5-1.2 5-3-2-3-5-3z', fill_rule='evenodd')))
add('capsule', P, 'Capsule', ink(p('M7 6a5 5 0 0 1 10 0v12a5 5 0 0 1-10 0z')), solid(p('M7 6a5 5 0 0 1 10 0v12a5 5 0 0 1-10 0z')))
add('wedge', P, 'Wedge', ink(p('M3 18V9l10-6 8 6v9l-10 4zM3 9l8 13M13 3l-2 19M3 18l18 0')), solid(p('M3 9 13 3l8 6v9l-10 4-8-4z')) + detail(p('M3 9l8 13M13 3l-2 19', stroke='#252735')))
add('circle', P, 'Circle', ink(c(12, 12, 9), p('M3 12h18', stroke_dasharray='2 2')), solid(c(12, 12, 9)))
badge('cube', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'add_primitive', P, 'Add Primitive')

S = 'Selection & transform'
add('select_box', S, 'Box Select', ink(r(3, 3, 18, 18, 1, stroke_dasharray='2 2'), p('M7 8v8l2-2 2 3 2-1-2-3 3-.5z')), solid(p('M3 3h3v2H5v2H3zm7 0h4v2h-4zm8 0h3v4h-2V5h-1zM3 10h2v4H3zm16 0h2v4h-2zM3 18h2v1h2v2H3zm7 1h4v2h-4zm8 0h1v-1h2v3h-3zM7 8v9l3-2 2 3 2-1-2-3 3-1z')))
add('select_lasso', S, 'Lasso Select', ink(p('M7 5C3 4 2 13 6 16s13 3 14-4-7-10-11-4-4 14 2 12'), p('M5 18q-1 4 3 3')), solid(p('M7 4C2 3 1 14 6 17c3 2 12 4 15-4 2-7-6-12-11-6-5 6-5 14 0 14l1-2c-3 0-3-6 1-11 4-5 9 0 7 5-2 5-10 4-12 2C4 13 5 6 8 6z')))
add('select_brush', S, 'Brush Select', ink(p('M5 18c2-2 4-1 5-4l7-10 3 3-10 8c-2 1-1 4-5 4zM3 21c4 0 5-1 6-3')), solid(p('M17 2l5 5-10 9c-1 1-1 4-4 5H2c2-2 2-4 4-6z')))
add('cursor_3d', S, '3D Cursor', ink(c(12, 12, 6), p('M12 2v6m0 8v6M2 12h6m8 0h6'), c(12, 12, 1)), solid(c(12, 12, 2), r(11, 1, 2, 7), r(11, 16, 2, 7), r(1, 11, 7, 2), r(16, 11, 7, 2)) + ink(c(12, 12, 6)))
add('move', S, 'Move', ink(p('M12 20V4m-3 3 3-3 3 3M4 12h16m-3-3 3 3-3 3M4 12l3-3m-3 3 3 3')), solid(p('M10.5 3 12 1l1.5 2L17 7l-2 2-2-2v3h3V7l4 5-4 5v-3h-3v3l2-2 2 2-5 4-5-4 2-2 2 2v-3H8v3l-4-5 4-5v3h3V7L9 9 7 7z')))
add('rotate', S, 'Rotate', ink(p('M18 7a8 8 0 1 0 2 8M17 3l2 4-4 1'), p('M12 9v3l3 2')), solid(p('M12 2a10 10 0 1 0 9.5 13h-3.2A7 7 0 1 1 16 5.5L13.5 8 21 9l-1-7-2 2A10 10 0 0 0 12 2z')))
add('scale', S, 'Scale', ink(r(3, 13, 8, 8, 1), r(14, 3, 7, 7, 1), p('M9 15 17 7m-4 0h4v4')), solid(r(2, 13, 9, 9, 1), r(14, 2, 8, 8, 1), p('M11 13 16 8l-2 0V6h6v6h-2v-2l-5 5z')))
add('transform', S, 'Universal Transform', ink(r(5, 5, 14, 14, 1), c(5, 5, 1), c(19, 5, 1), c(5, 19, 1), c(19, 19, 1), p('M12 2v20M2 12h20')), solid(r(3, 3, 4, 4, 1), r(17, 3, 4, 4, 1), r(3, 17, 4, 4, 1), r(17, 17, 4, 4, 1)) + ink(p('M5 5h14v14H5zM12 2v20M2 12h20')))
add('mode_object', S, 'Object Mode', *[ICONS['cube'][i] for i in (2, 3)])
add('mode_edit', S, 'Edit Mode', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), c(3, 7, 1.4), c(21, 7, 1.4), c(12, 22, 1.4)), solid(p(CUBE)) + solid(c(3, 7, 2), c(21, 7, 2), c(12, 22, 2)))
add('select_vertex', S, 'Select Point', ink(p('M4 17 11 4l9 11z'), c(11, 4, 2), c(4, 17, 2), c(20, 15, 2)), detail(p('M4 17 11 4l9 11z')) + solid(c(11, 4, 3), c(4, 17, 3), c(20, 15, 3)))
add('select_edge', S, 'Select Edge', ink(p('M4 17 11 4l9 11z'), p('M4 17 20 15', stroke_width='3.2'), c(4, 17, 1.5), c(20, 15, 1.5)), detail(p('M4 17 11 4l9 11z')) + solid(p('M3 15.5 21 13.5l.4 3L3.4 18.5z'), c(4, 17, 2), c(20, 15, 2)))
add('select_face', S, 'Select Face', ink(p(FACE), p('M8 9 16 8 17 16 9 17z')), solid(p(FACE)))
add('select_all', S, 'Select All', ink(r(3, 3, 18, 18, 2, stroke_dasharray='2 2'), p('M7 12l3 3 7-7')), solid(p('M2 2h4v2H4v2H2zm8 0h4v2h-4zm8 0h4v4h-2V4h-2zM2 10h2v4H2zm18 0h2v4h-2zM2 18h2v2h2v2H2zm8 2h4v2h-4zm8 0h2v-2h2v4h-4zM7 11l3 3 7-7 2 2-9 9-5-5z')))
badge('select_box', p('M15 18h6'), r(15, 17, 6, 2), 'deselect_all', S, 'Deselect All')
badge('select_box', p('M15 15l6 6m0-6-6 6'), p('M16 14l2 2 2-2 1 1-2 2 2 2-1 1-2-2-2 2-1-1 2-2-2-2z'), 'invert_selection', S, 'Invert Selection')
badge('select_vertex', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'grow_selection', S, 'Grow Selection')
badge('select_vertex', p('M15 18h6'), r(15, 17, 6, 2), 'shrink_selection', S, 'Shrink Selection')
add('select_linked', S, 'Select Linked', ink(c(5, 6, 2), c(19, 6, 2), c(12, 18, 2), p('M7 6h10M6 8l5 8m7-8-5 8')), solid(c(5, 6, 3), c(19, 6, 3), c(12, 18, 3), p('M7 5h10v2H7zM7 8l5 8-2 1-5-8zm10 0 2 1-5 8-2-1z')))
add('snap_magnet', S, 'Magnet Snap', ink(p('M5 3v9a7 7 0 0 0 14 0V3h-4v9a3 3 0 0 1-6 0V3zM5 7h4m6 0h4')), solid(p('M4 2h6v10a2 2 0 0 0 4 0V2h6v10a8 8 0 0 1-16 0z')))
add('proportional_editing', S, 'Proportional Editing', ink(c(12, 12, 2), c(12, 12, 6, stroke_dasharray='3 2'), c(12, 12, 10, stroke_dasharray='2 3')), solid(c(12, 12, 3), p('M12 5a7 7 0 1 1 0 14 7 7 0 0 1 0-14zm0 2a5 5 0 1 0 0 10 5 5 0 0 0 0-10z', fill_rule='evenodd')))
add('pivot_median', S, 'Median Pivot', ink(r(4, 4, 16, 16, 1), p('M12 2v20M2 12h20'), c(12, 12, 2)), solid(c(12, 12, 3), r(3, 3, 3, 3, 1), r(18, 3, 3, 3, 1), r(3, 18, 3, 3, 1), r(18, 18, 3, 3, 1)) + detail(p('M12 6v12M6 12h12')))
add('orientation_global', S, 'Global Axes', ink(c(12, 12, 2), p('M12 10V2m-2 3 2-3 2 3M14 12h8m-3-2 3 2-3 2M10 14l-6 6m0-4v4h4')), solid(c(12, 12, 3), p('M11 1h2v8h-2zM15 11h8v2h-8zM9 14l1.5 1.5L5 21l-1.5-1.5z')))

M = 'Mesh modeling'
add('extrude', M, 'Extrude', ink(p('M3 16 12 20l9-4-9-4zM3 16V9l9 4 9-4v7M3 9l9-4 9 4M12 13v7'), p('M12 10V2m-3 3 3-3 3 3')), solid(p('M3 9 12 5l9 4-9 4zM3 15l9 4 9-4v3l-9 4-9-4z')) + solid(p('M11 2h2v9h-2zM9 5l3-3 3 3z')))
add('extrude_individual', M, 'Extrude Individual', ink(p('M2 19 7 21l5-2-5-2zM12 19l5 2 5-2-5-2zM2 12l5 2 5-2-5-2zM12 12l5 2 5-2-5-2zM7 17v-3m10 3v-3M7 8V3m10 5V3')), solid(p('M2 12 7 9l5 3-5 3zM12 12l5-3 5 3-5 3zM2 18l5-2 5 2-5 3zM12 18l5-2 5 2-5 3zM6 2h2v6H6zm10 0h2v6h-2z')))
add('pushpull', M, 'Push/Pull', ink(p('M3 10 12 6l9 4-9 4zM3 10v8l9 4 9-4v-8M12 14v8M17 2v6m-2-4 2-2 2 2M7 22v-6m-2 4 2 2 2-2')), solid(p('M3 10 12 6l9 4-9 4zM3 16l9 4 9-4v3l-9 4-9-4zM16 1h2v7h-2zM15 4l2-3 2 3z')))
add('inset', M, 'Inset', ink(p(FACE), p('M8 9 16 8l1 7-8 2z'), p('M4 7l4 2m11-4-3 3m5 10-4-3m-11 5 3-3')), solid(p(FACE)) + ink(p('M8 9 16 8l1 7-8 2z', stroke='#252735')))
add('bevel', M, 'Round Edge', ink(p('M3 17V7l4-4h10l4 4v10l-4 4H7zM3 7h4V3m10 0v4h4M3 17h4v4m10 0v-4h4')), solid(p('M7 2h10l5 5v10l-5 5H7l-5-5V7z')) + ink(p('M3 8h4V3m10 0v5h4M3 16h4v5m10 0v-5h4', stroke='#252735')))
add('loop_cut', M, 'Loop Cut', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M7.5 4.5v15M16.5 4.5v15M7.5 9.5l9 0', stroke_dasharray='2 2')), solid(p(CUBE)) + ink(p('M7.5 4.5v15M16.5 4.5v15M7.5 9.5l9 0', stroke='#252735')))
add('knife', M, 'Knife', ink(p('M3 20 14 9l5-6 2 2-6 5L4 21zM13 10l2 2M4 21l4-1')), solid(p('M2 21 13 10l6-8 3 3-8 6L4 22z')))
add('tool_poly_pen', M, 'Poly Pen', ink(p('M3 21V12l7-4M3 21h7'), c(3, 21, 1), c(3, 12, 1), c(10, 8, 1), p('m13 15 6.5-6.5 2 2L15 17l-3 1z')), solid(p('M2 22V11.3l8-4.6 1 1.7-7 4V20h6v2z'), p('m12.5 14.5 7-7 3 3-7 7-4 1z')))
add('slice', M, 'Slice', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M2 13 22 10', stroke_dasharray='2 2')), solid(p(CUBE)) + ink(p('M2 13 22 10', stroke='#252735', stroke_width='2.2')))
add('subdivide', M, 'Subdivide', ink(r(3, 3, 18, 18, 1), p('M12 3v18M3 12h18')), solid(r(3, 3, 8, 8, 1), r(13, 3, 8, 8, 1), r(3, 13, 8, 8, 1), r(13, 13, 8, 8, 1)))
add('merge', M, 'Merge Center', ink(c(3, 4, 2), c(21, 4, 2), c(3, 20, 2), c(21, 20, 2), c(12, 12, 2), p('M5 6l5 5M19 6l-5 5M5 18l5-5m9 5-5-5')), solid(c(12, 12, 3), c(3, 4, 2), c(21, 4, 2), c(3, 20, 2), c(21, 20, 2)) + detail(p('M5 6l5 5M19 6l-5 5M5 18l5-5m9 5-5-5')))
add('weld', M, 'Merge by Distance', ink(c(5, 6, 2), c(19, 6, 2), c(12, 19, 2), p('M6 7 12 16l6-9M6 6h12')), solid(c(12, 12, 3), c(5, 6, 2), c(19, 6, 2), c(12, 19, 2)) + detail(p('M6 7 12 16l6-9')))
add('connect', M, 'Connect', ink(p('M3 5l8 4v11l-8-4zM13 4l8 4v11l-8-4zM11 9l2-5M11 20l2-5M3 5l10-1M3 16l10-1')), solid(p('M2 4 10 8v13l-8-4zM14 3l8 4v13l-8-4zM10 8l4-5v3l-4 6zM10 18l4-5v4l-4 4z')))
add('make_face', M, 'Make Face', ink(p('M3 18 9 4l12 2-4 15z'), c(3, 18, 1), c(9, 4, 1), c(21, 6, 1), c(17, 21, 1)), solid(p('M3 18 9 4l12 2-4 15z')))
add('dissolve', M, 'Dissolve', ink(r(3, 3, 18, 18, 1), p('M12 3v7m0 4v7M3 12h7m4 0h7'), p('M10 10l4 4m0-4-4 4')), solid(p('M3 3h8v8H3zM13 3h8v8h-8zM3 13h8v8H3zM13 13h8v8h-8z')) + ink(p('M9 9l6 6m0-6-6 6', stroke='#252735')))
add('flip_diagonal', M, 'Flip Diagonal', ink(p('M3 4 21 4l-3 16H5zM4 5l14 14m2-14L5 19'), p('m14 5 6 0-1 6')), solid(p('M3 4h18l-3 16H5z')) + ink(p('M4 5l14 14m2-14L5 19', stroke='#252735')))
add('flip_normals', M, 'Flip Normals', ink(p(FACE), p('M12 12V3m-3 3 3-3 3 3M16 16v6m-3-3 3 3 3-3')), solid(p(FACE)) + solid(p('M11 1h2v10h-2zM8 6l4-5 4 5zM15 15h2v6h-2zM12 18l4 5 4-5z')))
add('mirror', M, 'Mirror', ink(p('M12 2v20', stroke_dasharray='2 2'), p('M3 7l6-3v16l-6-3zM21 7l-6-3v16l6-3z')), solid(p('M2 7 9 3v18l-7-4zM22 7l-7-4v18l7-4zM11 2h2v20h-2z')))
add('symmetrize', M, 'Symmetrize', ink(p('M12 2v20', stroke_dasharray='2 2'), p('M3 7l6-3v16l-6-3zM15 4l6 3v10l-6 3z'), p('m10 12 4 0m-2-2 2 2-2 2')), solid(p('M2 7 9 3v18l-7-4zM15 3l7 4v10l-7 4zM11 2h2v20h-2z')))
add('separate', M, 'Separate Selection', ink(p('M3 7l6-3 5 3-6 3zM3 7v10l5 3 6-3V7M16 10l5-3v10l-5 3z'), p('M11 12h7m-3-3 3 3-3 3')), solid(p('M2 7 9 3l5 3v12l-6 3-6-4zM17 9l5-3v12l-5 3z')))
add('revolve', M, 'Revolve', ink(p('M8 5v14M8 5q11-1 10 7t-10 7M4 4v16', stroke_dasharray='3 2'), p('M18 8l3 4-3 3')), solid(p('M7 4h3v16H7zM10 4c8-1 12 3 12 8s-4 9-12 8v-3c6 0 9-2 9-5s-3-5-9-5z')))
add('spin', M, 'Spin', ink(p('M3 18 9 20l3-5-6-2zM12 15c8 2 11-6 5-10M15 4l2 1 1 3')), solid(p('M2 18 6 12l7 3-4 6zM12 15c7 0 10-6 5-10l-2 1c3 3 1 7-3 7zM15 3l5 2-1 5-2-3z')))
UNION = 'M12 6.8A6 6 0 1 0 12 17.2A6 6 0 1 0 12 6.8z'
BITE = 'M12 6.8A6 6 0 1 0 12 17.2A6 6 0 0 1 12 6.8z'
LENS = 'M12 6.8A6 6 0 0 1 12 17.2A6 6 0 0 1 12 6.8z'
add('boolean_fuse', M, 'Fuse', ink(p(UNION)), solid(p(UNION)))
add('boolean_cut', M, 'Cut', ink(p(BITE), p('M12 6.8A6 6 0 1 1 12 17.2', stroke_dasharray='2 2.4')), solid(p(BITE)) + ink(p('M12 6.8A6 6 0 1 1 12 17.2', stroke_dasharray='2 2.4')))
OUTER = ink(p('M12 6.8A6 6 0 1 0 12 17.2', stroke_dasharray='2 2.4'), p('M12 6.8A6 6 0 1 1 12 17.2', stroke_dasharray='2 2.4'))
add('boolean_intersect', M, 'Intersect', ink(p(LENS)) + OUTER, solid(p(LENS)) + OUTER)
add('boolean_join', M, 'Join', ink(c(6.5, 12, 4.5), c(17.5, 12, 4.5), p('M11 12h2')), solid(c(6.5, 12, 4.5), c(17.5, 12, 4.5)) + ink(p('M10 12h4')))

U = 'UV & materials'
add('uv_editor', U, 'UV Editor', ink(r(2, 2, 20, 20, 1), p('M2 12h20M12 2v20M4 4l6 3 1 3-7 1zM14 14l7 1-3 6-5-2z')), solid(r(2, 2, 20, 20, 1)) + ink(p('M2 12h20M12 2v20M4 4l6 3 1 3-7 1zM14 14l7 1-3 6-5-2z', stroke='#252735')))
add('uv_island', U, 'UV Island', ink(r(2, 2, 20, 20, 1), p('M5 8 11 5l7 4-3 8-9 2z'), c(11, 5, 1), c(18, 9, 1), c(15, 17, 1), c(6, 19, 1)), solid(p('M4 7 11 4l8 5-3 9-10 3z')))
add('uv_unwrap', U, 'Unwrap', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M3 18l-2 3m20-3 2 3')), solid(p('M3 7 12 2l9 5-9 5zM3 8v8l9 5v-8zM13 13l8-5v8l-8 5z')))
add('uv_pack', U, 'Pack Islands', ink(r(2, 2, 20, 20, 1), p('M4 4h7v6H4zM13 4h7v9h-7zM4 12h7v8H4zM13 15h7v5h-7z')), solid(r(3, 3, 8, 7, 1), r(13, 3, 8, 10, 1), r(3, 12, 8, 9, 1), r(13, 15, 8, 6, 1)))
add('uv_seam', U, 'UV Seam', ink(p('M3 4 10 2l6 5-3 14-9-5zM10 2l6 5 5-3v15l-8 2'), p('M10 2l3 19', stroke_dasharray='2 2')), solid(p('M3 4 10 2l3 19-9-5zM16 7l5-3v15l-8 2z')) + ink(p('M10 2l3 19', stroke='#252735', stroke_dasharray='2 2')))
add('uv_stitch', U, 'Stitch Seams', ink(p('M3 4v16h7V4zM14 4v16h7V4zM10 8h4m-4 4h4m-4 4h4'), c(12, 8, 1), c(12, 16, 1)), solid(r(3, 4, 7, 16, 1), r(14, 4, 7, 16, 1), r(10, 7, 4, 2), r(10, 15, 4, 2)))
add('uv_relax', U, 'Relax UVs', ink(r(2, 2, 20, 20, 1), p('M4 8c4-4 6 4 10 0s5-2 6-2M4 15c4-4 6 4 10 0s5-2 6-2')), solid(p('M3 2h18v20H3z')) + ink(p('M4 8c4-4 6 4 10 0s5-2 6-2M4 15c4-4 6 4 10 0s5-2 6-2', stroke='#252735')))
add('uv_project_view', U, 'Project From View', ink(p('M2 8l10-5 10 5v10l-10 5-10-5zM12 3v20'), p('M3 10h18m-18 6h18', stroke_dasharray='2 2')), solid(p('M2 8l10-5 10 5v10l-10 5-10-5z')) + ink(p('M12 3v20M3 10h18m-18 6h18', stroke='#252735')))
badge('reference_image', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'uv_project_reference', U, 'Project From Reference')
add('uv_checker', U, 'UV Checker', ink(r(2, 2, 20, 20, 1), p('M12 2v20M2 12h20')), solid(r(2, 2, 10, 10), r(12, 12, 10, 10)) + ink(r(2, 2, 20, 20, 0)))
add('uv_move', U, 'Move UV', ink(r(3, 3, 18, 18, 1), p('M12 6v12m-3-3 3 3 3-3M6 12h12m-3-3 3 3-3 3')), solid(r(3, 3, 18, 18, 1)) + ink(p('M12 6v12m-3-3 3 3 3-3M6 12h12m-3-3 3 3-3 3', stroke='#252735')))
add('uv_rotate', U, 'Rotate UV', ink(r(3, 3, 18, 18, 1), p('M18 10a6 6 0 1 0-1 6M16 6l2 4-4 1')), solid(r(3, 3, 18, 18, 1)) + ink(p('M18 10a6 6 0 1 0-1 6M16 6l2 4-4 1', stroke='#252735')))
add('uv_scale', U, 'Scale UV', ink(r(3, 3, 18, 18, 1), r(6, 10, 8, 8, 1), p('M12 12l6-6m-4 0h4v4')), solid(r(3, 3, 18, 18, 1)) + ink(r(6, 10, 8, 8, 1, stroke='#252735'), p('M12 12l6-6m-4 0h4v4', stroke='#252735')))
add('material', U, 'Material', ink(c(12, 12, 10), p('M5 17c5 1 11-1 15-7M6 5c3 5 8 10 15 10'), c(9, 9, 2)), solid(c(12, 12, 10)) + ink(p('M5 17c5 1 11-1 15-7M6 5c3 5 8 10 15 10', stroke='#252735')))
add('texture', U, 'Texture', ink(r(2, 3, 20, 18, 1), p('M2 10h20M8 3v7M15 10v11M2 16h13'), c(18, 6, 1)), solid(r(2, 3, 20, 18, 1)) + ink(p('M2 10h20M8 3v7M15 10v11M2 16h13', stroke='#252735')))
add('shader', U, 'Shader', ink(c(5, 6, 2), c(19, 6, 2), c(12, 18, 2), p('M7 6h10M6 8l5 8m7-8-5 8')), solid(c(5, 6, 3), c(19, 6, 3), c(12, 18, 3)) + detail(p('M7 6h10M6 8l5 8m7-8-5 8')))
add('normal_map', U, 'Normal Map', ink(r(3, 3, 18, 18, 1), p('M12 17V7m-3 3 3-3 3 3M5 17l7-5 7 5')), solid(r(3, 3, 18, 18, 1)) + ink(p('M12 17V7m-3 3 3-3 3 3M5 17l7-5 7 5', stroke='#252735')))
add('roughness', U, 'Roughness', ink(c(12, 12, 9), p('M5 7l3 4 3-3 4 5 4-4M4 15l3 2 3-3 4 3 4-2')), solid(p('M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18z')) + ink(p('M5 7l3 4 3-3 4 5 4-4M4 15l3 2 3-3 4 3 4-2', stroke='#252735')))
add('metallic', U, 'Metallic', ink(p('M3 5h18v14H3zM3 10h18M8 5l-2 5m6-5-2 5m6-5-2 5m6-5-2 5')), solid(p('M2 4h20v16H2z')) + detail(p('M2 10h20M8 4l-2 6m6-6-2 6m6-6-2 6m6-6-2 6', stroke='#252735')))
add('opacity', U, 'Opacity', ink(p('M12 2c-2 4-7 9-7 13a7 7 0 0 0 14 0c0-4-5-9-7-13zM6 15c3-1 6 1 12 0')), solid(p('M12 1C9 6 4 11 4 15a8 8 0 0 0 16 0c0-4-5-9-8-14z')))

B = 'Painting'
add('paint_brush', B, 'Paint Brush', ink(p('M4 20c3 1 6-1 6-4L20 5l-2-2L7 13c-3 0-5 3-3 7zM8 14l2 2')), solid(p('M3 20c4 1 7-1 7-4L21 5l-3-3L7 13c-4 0-6 4-4 7z')))
add('paint_eraser', B, 'Eraser', ink(p('M9 3h6l7 7-10 11H6L2 17zM5 14l7 7M3 22h18')), solid(p('M9 2h6l8 8-10 11H6L1 17z')) + detail(p('M5 14l7 7M3 22h18')))
add('paint_fill', B, 'Fill', ink(p('m4 4 3-2 12 12-8 8-9-9 8-8M3 16h18'), p('M20 10c-1 2-2 3-2 4a2 2 0 0 0 4 0c0-1-1-2-2-4z')), solid(p('M7 2 21 16l-9 7L2 13l8-8zM20 8c-2 3-3 5-3 6a3 3 0 0 0 6 0c0-1-1-3-3-6z')))
add('paint_picker', B, 'Eyedropper', ink(p('M13 5l6-3 3 3-3 6-2-2L7 19l-5 3 3-5L15 7zM3 17l4 4')), solid(p('M19 1 23 5l-4 7-2-2L7 21l-6 2 2-6L14 6l-2-2z')))
add('paint_line', B, 'Paint Line', ink(p('M4 19 20 5'), c(4, 19, 2), c(20, 5, 2)), solid(p('M3 18 19 4l2 2L5 20z'), c(4, 19, 3), c(20, 5, 3)))
add('paint_rect', B, 'Paint Rectangle', ink(r(3, 4, 18, 16, 1), c(3, 4, 1), c(21, 20, 1)), solid(r(3, 4, 18, 16, 1)))
add('paint_smudge', B, 'Smudge', ink(p('M5 20c4-3 3-7 7-8s6-5 3-9M7 16c2-5 8-2 11-8M4 21h12')), solid(p('M4 21c5-4 2-7 8-9 5-2 7-5 3-10l3-1c5 7 0 11-5 13-4 2-2 6-7 9z')))
add('paint_clone', B, 'Clone Brush', ink(p('M4 20c3 1 6-1 6-4L20 5l-2-2L7 13c-3 0-5 3-3 7z'), p('M17 15v7m-3-3h7')), solid(p('M3 20c4 1 7-1 7-4L21 5l-3-3L7 13c-4 0-6 4-4 7zM16 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z')))
add('paint_layer', B, 'Paint Layer', ink(p('M3 8 12 3l9 5-9 5zM3 13l9 5 9-5M3 18l9 5 9-5'), p('M15 5l2 2')), solid(p('M2 8 12 2l10 6-10 6zM2 13l10 6 10-6v3l-10 6L2 16z')))
badge('paint_layer', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'new_layer', B, 'New Layer')
badge('paint_layer', p('M15 18h6'), r(15, 17, 6, 2), 'delete_layer', B, 'Delete Layer')
add('paint_mask', B, 'Layer Mask', ink(r(3, 3, 18, 18, 2), c(12, 12, 6), p('M12 6a6 6 0 0 1 0 12')), solid(r(3, 3, 18, 18, 2)) + solid(p('M12 5a7 7 0 0 0 0 14z', fill='#252735')))
add('paint_palette', B, 'Color Palette', ink(p('M12 2a10 10 0 0 0 0 20h1c2 0 3-2 2-3-.5-1 0-2 1-2h3c2 0 3-1 3-4A10 10 0 0 0 12 2z'), c(7, 8, 1), c(12, 6, 1), c(17, 9, 1), c(8, 15, 1)), solid(p('M12 2a10 10 0 0 0 0 20h1c2 0 3-2 2-3-.5-1 0-2 1-2h3c2 0 3-1 3-4A10 10 0 0 0 12 2z')) + solid(c(7, 8, 1.5, fill='#252735'), c(12, 6, 1.5, fill='#252735'), c(17, 9, 1.5, fill='#252735'), c(8, 15, 1.5, fill='#252735')))
add('paint_decal', B, 'Decal', ink(r(3, 3, 18, 18, 1), p('M7 17l3-7 4 4 3-6 2 9z'), c(8, 7, 1)), solid(r(3, 3, 18, 18, 1)) + ink(p('M7 17l3-7 4 4 3-6 2 9z', stroke='#252735')))
badge('paint_decal', p('M14 18h8m-2-2 2 2-2 2'), p('M14 17h7l-2-2 1-1 4 4-4 4-1-1 2-2h-7z'), 'bake_decal', B, 'Bake Decal')
badge('reference_image', p('M14 18h8m-2-2 2 2-2 2'), p('M14 17h7l-2-2 1-1 4 4-4 4-1-1 2-2h-7z'), 'bake_reference', B, 'Bake Reference')
add('paint_2d', B, 'Paint in 2D', ink(r(3, 3, 18, 18, 1), p('M7 17 16 7m-3 0h3v3M6 18l4-1')), solid(r(3, 3, 18, 18, 1)) + ink(p('M7 17 16 7m-3 0h3v3M6 18l4-1', stroke='#252735')))
add('paint_3d', B, 'Paint on Model', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M6 16c3-5 7-3 8-5')), solid(p(CUBE)) + ink(p('M6 16c3-5 7-3 8-5', stroke='#252735', stroke_width='2.5')))

V = 'Viewport & scene'
add('camera', V, 'Camera', ink(r(2, 6, 13, 12, 2), p('m15 9 7-4v14l-7-4z'), c(8.5, 12, 2)), solid(r(2, 6, 13, 12, 2), p('m15 9 7-4v14l-7-4z')) + detail(c(8.5, 12, 2, stroke='#252735')))
add('view_perspective', V, 'Perspective View', ink(p('M4 6 17 3l4 4-4 13-13-3zM4 6l9 5 8-4M13 11l4 9'), p('M2 2h3m-3 0v3')), solid(p('M4 6 17 3l4 4-4 13-13-3z')) + detail(p('M4 6l9 5 8-4M13 11l4 9', stroke='#252735')))
add('view_orthographic', V, 'Orthographic View', ink(p('M3 7 12 2l9 5v10l-9 5-9-5zM3 7l9 5 9-5M12 12v10')), solid(p(CUBE)) + detail(p('M3 7l9 5 9-5M12 12v10', stroke='#252735')))
add('view_front', V, 'Front View', ink(r(4, 4, 16, 16, 1), p('M4 10h16M10 10v10'), c(16, 15, 1)), solid(r(4, 4, 16, 16, 1)) + detail(p('M4 10h16M10 10v10', stroke='#252735')))
add('view_top', V, 'Top View', ink(r(4, 4, 16, 16, 1), p('M4 4l16 16M20 4 4 20'), c(12, 12, 2)), solid(r(4, 4, 16, 16, 1)) + detail(p('M4 4l16 16M20 4 4 20', stroke='#252735')))
add('view_right', V, 'Right View', ink(p('M5 4h11l3 4v12H5zM16 4v5h3M5 13h14')), solid(p('M5 4h11l3 4v12H5z')) + detail(p('M16 4v5h3M5 13h14', stroke='#252735')))
add('view_back', V, 'Back View', ink(r(4, 4, 16, 16, 1), p('M4 14h16M14 4v10'), c(8, 9, 1)), solid(r(4, 4, 16, 16, 1)) + detail(p('M4 14h16M14 4v10', stroke='#252735')))
add('view_bottom', V, 'Bottom View', ink(r(4, 4, 16, 16, 1), p('M4 4l16 16M20 4 4 20M4 17h16')), solid(r(4, 4, 16, 16, 1)) + detail(p('M4 4l16 16M20 4 4 20M4 17h16', stroke='#252735')))
add('view_left', V, 'Left View', ink(p('M8 4h11v16H5V8zM5 9h3V4M5 13h14')), solid(p('M8 4h11v16H5V8z')) + detail(p('M5 9h3V4M5 13h14', stroke='#252735')))
add('frame_selection', V, 'Frame Selection', ink(p('M3 9V3h6m6 0h6v6m0 6v6h-6m-6 0H3v-6'), c(12, 12, 3)), solid(p('M2 2h8v2H4v6H2zm12 0h8v8h-2V4h-6zM2 14h2v6h6v2H2zm18 0h2v8h-8v-2h6z'), c(12, 12, 3)))
badge('frame_selection', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'frame_all', V, 'Frame All')
add('orbit', V, 'Orbit', ink(c(12, 12, 6), p('M3 12c0-5 18-5 18 0s-18 5-18 0M18 5l3-1-1 3')), solid(p('M12 4a8 8 0 1 1 0 16 8 8 0 0 1 0-16zm0 3a5 5 0 1 0 0 10 5 5 0 0 0 0-10z', fill_rule='evenodd')) + ink(p('M3 12c0-5 18-5 18 0', stroke_width='2')))
add('pan', V, 'Pan', ink(p('M7 14V9a2 2 0 0 1 4 0v3-7a2 2 0 0 1 4 0v6-3a2 2 0 0 1 4 0v8c0 4-4 6-8 6H9c-3 0-5-3-7-7-1-3 2-5 5-1z')), solid(p('M11 3a3 3 0 0 1 6 0v3a3 3 0 0 1 5 2v8c0 5-4 8-9 8h-3c-4 0-6-4-8-8-2-4 2-7 5-5V8a3 3 0 0 1 4-3z')))
add('zoom', V, 'Zoom', ink(c(10, 10, 7), p('M15 15l6 6M10 6v8M6 10h8')), solid(p('M10 1a9 9 0 0 1 7.1 14.5L23 21l-2 2-5.5-5.9A9 9 0 1 1 10 1zm-1 4v4H5v2h4v4h2v-4h4V9h-4V5z', fill_rule='evenodd')))
add('shading_wireframe', V, 'Wireframe', ink(c(12, 12, 9), p('M3 12h18M12 3c-5 5-5 13 0 18m0-18c5 5 5 13 0 18')), solid(c(12, 12, 9)) + detail(p('M3 12h18M12 3c-5 5-5 13 0 18m0-18c5 5 5 13 0 18', stroke='#252735')))
add('shading_solid', V, 'Solid Shading', ink(c(12, 12, 9), p('M12 3a9 9 0 0 0 0 18')), solid(c(12, 12, 9)))
add('shading_material', V, 'Material Preview', ink(c(12, 12, 9), p('M12 3a9 9 0 0 0 0 18'), c(17, 7, 2)), solid(p('M12 3a9 9 0 1 1 0 18z')) + solid(c(17, 7, 3, fill='#252735')))
add('shading_rendered', V, 'Rendered Preview', ink(c(12, 12, 7), p('M12 1v3m0 16v3M1 12h3m16 0h3M4 4l2 2m12 12 2 2M20 4l-2 2M6 18l-2 2')), solid(c(12, 12, 6), r(11, 1, 2, 3), r(11, 20, 2, 3), r(1, 11, 3, 2), r(20, 11, 3, 2), p('M4 3 3 4l2 3 2-2zM20 3l1 1-2 3-2-2zM4 21l-1-1 2-3 2 2zM20 21l1-1-2-3-2 2z')))
add('wire_overlay', V, 'Wire Overlay', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M3 12l9 5 9-5', stroke_dasharray='2 2')), solid(p(CUBE)) + detail(p('M3 7l9 5 9-5M12 12v10M3 12l9 5 9-5', stroke='#252735')))
add('xray', V, 'X-Ray', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M3 17l9-5 9 5M12 2v10', stroke_dasharray='2 2')), solid(p(CUBE, fill_opacity='.55')) + detail(p('M3 7l9 5 9-5M12 12v10M3 17l9-5 9 5')))
add('grid_overlay', V, 'Grid', ink(p('M2 7 12 3l10 4-10 14L2 7zM5 11h14M8 16h8M8 5l4 16m4-16-4 16')), solid(p('M2 7 12 3l10 4-10 14z', fill_opacity='.45')) + detail(p('M5 11h14M8 16h8M8 5l4 16m4-16-4 16')))
add('overlays', V, 'Overlays', ink(p('M3 7 12 2l9 5-9 5zM3 12l9 5 9-5M3 17l9 5 9-5')), solid(p('M2 7 12 1l10 6-10 6zM2 12l10 6 10-6v3l-10 6-10-6z')))
add('hierarchy', V, 'Scene Hierarchy', ink(r(2, 2, 7, 6, 1), r(15, 9, 7, 6, 1), r(15, 17, 7, 6, 1), p('M6 8v13h9M6 12h9')), solid(r(2, 2, 7, 6, 1), r(15, 9, 7, 6, 1), r(15, 17, 7, 6, 1), r(5, 8, 2, 13), r(6, 11, 9, 2), r(6, 19, 9, 2)))
badge('folder', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'collection', V, 'Collection')
add('asset_library', V, 'Asset Library', ink(r(2, 3, 20, 18, 2), p('M8 3v18M8 10h14M3 10h5'), p('M13 15l3-2 3 2-3 2z')), solid(r(2, 3, 20, 18, 2)) + ink(p('M8 3v18M8 10h14M3 10h5M13 15l3-2 3 2-3 2z', stroke='#252735')))
badge('asset_library', p('M15 18h6m-3-3v6'), p('M17 14h2v3h3v2h-3v3h-2v-3h-3v-2h3z'), 'save_asset', V, 'Save Asset')
add('axis_gizmo', V, 'Axis Gizmo', ink(c(12, 12, 2), p('M12 10V2m-2 3 2-3 2 3M14 12h8m-3-2 3 2-3 2M10 14l-6 6m0-4v4h4')), solid(c(12, 12, 3), r(11, 1, 2, 8), r(15, 11, 8, 2), p('M9 14l1.5 1.5L5 21l-1.5-1.5z')))

A = 'Production & extension'
add('render', A, 'Render', ink(r(2, 6, 20, 15, 2), p('M6 6l3-4h6l3 4'), c(12, 13, 4), p('M12 9v8m-4-4h8')), solid(r(2, 6, 20, 15, 2), p('M6 6l3-4h6l3 4z')) + detail(c(12, 13, 4, stroke='#252735')))
add('output', A, 'Output', ink(p('M4 3h16v12l-5 6H4zM15 15h5M8 8h8m-8 4h6')), solid(p('M4 3h16v12l-5 6H4z')) + detail(p('M15 15h5M8 8h8m-8 4h6', stroke='#252735')))
add('export_glb', A, 'Export GLB', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M17 2v6m-2-2 2 2 2-2')), solid(p(CUBE)) + solid(p('M16 1h2v5l2-2 1 1-4 4-4-4 1-1 2 2z')))
add('export_obj', A, 'Export OBJ', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7'), p('M17 2v6m-2-2 2 2 2-2')), solid(p(CUBE, fill_opacity='.7')) + ink(p('M3 7l9 5 9-5M12 12v10')))
add('plugin', A, 'Plugin', ink(p('M4 4h6v3a2 2 0 0 0 4 0V4h6v6h-3a2 2 0 0 0 0 4h3v6h-6v-3a2 2 0 0 0-4 0v3H4v-6h3a2 2 0 0 0 0-4H4z')), solid(p('M3 3h8v4a1 1 0 0 0 2 0V3h8v8h-4a1 1 0 0 0 0 2h4v8h-8v-4a1 1 0 0 0-2 0v4H3v-8h4a1 1 0 0 0 0-2H3z')))
add('script', A, 'Lua Script', ink(p('M5 2h9l5 5v15H5zM14 2v5h5'), p('M9 12l-3 3 3 3m6-6 3 3-3 3')), solid(p('M5 2h9l5 5v15H5z')) + ink(p('M9 12l-3 3 3 3m6-6 3 3-3 3', stroke='#252735')))
add('command_palette', A, 'Command Palette', ink(r(2, 4, 20, 16, 2), p('M6 9l3 3-3 3m6 0h6')), solid(r(2, 4, 20, 16, 2)) + ink(p('M6 9l3 3-3 3m6 0h6', stroke='#252735')))

F = 'Future roadmap'
add('timeline', F, 'Timeline', ink(r(2, 4, 20, 16, 1), p('M2 10h20M6 14v3m4-4v4m4-3v3m4-4v4'), c(12, 10, 2)), solid(r(2, 4, 20, 16, 1)) + ink(p('M2 10h20M6 14v3m4-4v4m4-3v3m4-4v4', stroke='#252735')) + solid(c(12, 10, 2)), 'planned')
add('keyframe', F, 'Keyframe', ink(p('M12 3 21 12l-9 9-9-9z')), solid(p('M12 2 22 12 12 22 2 12z')), 'planned')
add('bone', F, 'Bone', ink(c(5, 5, 2), c(19, 5, 2), c(5, 19, 2), c(19, 19, 2), p('M7 7l10 10m0-10L7 17')), solid(c(5, 5, 3), c(19, 5, 3), c(5, 19, 3), c(19, 19, 3), p('M7 7l10 10-2 2L5 9zM17 7 7 17l2 2L19 9z')), 'planned')
add('hair', F, 'Hair', ink(p('M4 21C2 13 3 4 12 3c10 0 12 11 8 18M5 13c4-7 8-7 12 0M6 17c5-6 9-6 12 0')), solid(p('M3 22C0 9 4 2 12 2c9 0 13 8 9 20h-3c3-12 0-17-6-17-6 0-9 6-6 17z')), 'planned')
add('cloth', F, 'Cloth', ink(p('M3 3h18v14l-4 4-5-4-5 4-4-4zM3 8c6-4 12 4 18 0')), solid(p('M3 3h18v14l-4 4-5-4-5 4-4-4z')) + ink(p('M3 8c6-4 12 4 18 0', stroke='#252735')), 'planned')
add('particles', F, 'Particles', ink(c(12, 12, 2), c(4, 5, 1), c(19, 5, 1), c(5, 20, 1), c(19, 19, 1), p('M12 8V3m4 7 3-3M9 15l-3 3')), solid(c(12, 12, 3), c(4, 5, 2), c(19, 5, 2), c(5, 20, 2), c(19, 19, 2)), 'planned')

add('measure', G, 'Measure', ink(p('M3 17 17 3l4 4L7 21zM7 17l-2-2m5-1-2-2m5-1-2-2m5-1-2-2')), solid(p('M3 17 17 3l4 4L7 21z')) + detail(p('M7 17l-2-2m5-1-2-2m5-1-2-2m5-1-2-2', stroke='#252735')))
add('annotate', G, 'Annotate', ink(p('M3 20l5-1L20 7l-3-3L5 16zM14 7l3 3M2 22h20')), solid(p('M3 20l5-1L20 7l-3-3L5 16z')) + detail(p('M2 22h20')))
add('step_forward', G, 'Step Forward', ink(p('M5 5 14 12 5 19zM18 5v14')), solid(p('M4 3 16 12 4 21z'), r(18, 3, 3, 18)))
add('step_backward', G, 'Step Backward', ink(p('m19 5-9 7 9 7zM6 5v14')), solid(p('m20 3-12 9 12 9z'), r(3, 3, 3, 18)))
add('jump_start', G, 'Jump to Start', ink(p('M4 4v16m3-8 12-8v16z')), solid(r(3, 3, 3, 18), p('M8 12 21 3v18z')))
add('jump_end', G, 'Jump to End', ink(p('M20 4v16m-3-8L5 4v16z')), solid(r(18, 3, 3, 18), p('M16 12 3 3v18z')))
add('modifier', M, 'Modifier', ink(p('M19 3a5 5 0 0 0-5 6L5 18a2 2 0 0 0 3 3l9-9a5 5 0 0 0 6-6l-4 4-3-3z')), solid(p('M19 2a6 6 0 0 0-6 7L4 18a3 3 0 0 0 4 4l10-10a6 6 0 0 0 5-7l-4 4-3-3z')))


def alias(key: str, target: str, label: str, group: str | None = None) -> None:
    category, _, outline, filled, status = ICONS[target]
    add(key, group or category, label, outline, filled, status)


# Existing IconId spellings are kept verbatim. Some IDs intentionally share art.
for key, target in {
    'annotate': 'draw_profile', 'measure': 'thickness',
    'reference_manager': 'reference_image', 'object_mesh': 'cube',
    'view_wire': 'shading_wireframe', 'view_solid': 'shading_solid',
    'view_material': 'shading_material', 'view_lit': 'shading_rendered',
    'paint': 'paint_brush', 'revolve_profile': 'revolve',
    'step_forward': 'chevron_right', 'step_backward': 'chevron_left',
    'jump_start': 'chevron_left', 'jump_end': 'chevron_right',
    'minimize': 'minus', 'maximize': 'paint_rect',
    'isolate': 'frame_selection', 'shade_flat': 'shading_solid',
    'shade_smooth': 'shading_material', 'normals_overlay': 'flip_normals',
    'backface_culling': 'eye_hidden', 'bounding_box': 'transform',
    'origin': 'cursor_3d', 'selection_center': 'pivot_median',
    'project_from_reference': 'uv_project_reference',
    'project_from_view': 'uv_project_view', 'unwrap_auto': 'uv_unwrap',
    'pack_islands': 'uv_pack', 'stitch': 'uv_stitch', 'relax': 'uv_relax',
    'tool_position': 'move', 'tool_profile': 'draw_profile',
    'tool_extrude': 'extrude', 'tool_pushpull': 'pushpull',
    'tool_inset': 'inset', 'tool_bevel': 'bevel', 'tool_knife': 'knife',
    'tool_loopcut': 'loop_cut', 'tool_slice': 'slice',
    'tool_connect': 'connect', 'tool_subdivide': 'subdivide',
    'tool_spin': 'spin', 'tool_dissolve': 'dissolve', 'tool_merge': 'merge',
    'trash': 'delete', 'select': 'select_box', 'primitives': 'add_primitive',
    'tool_duplicate': 'duplicate', 'tool_rotate': 'rotate',
    'tool_scale': 'scale', 'tool_transform': 'transform',
}.items():
    if key not in ICONS:
        alias(key, target, key.replace('_', ' ').title())

for number, source, label in [
    (1, 'transform', 'Tool'), (2, 'render', 'Render'), (3, 'output', 'Output'),
    (4, 'paint_layer', 'View Layer'), (5, 'hierarchy', 'Scene'),
    (6, 'shading_rendered', 'World'), (7, 'collection', 'Collection'),
    (8, 'cube', 'Object'), (9, 'modifier', 'Modifier'),
    (10, 'object_mesh', 'Data'), (11, 'material', 'Material'),
    (12, 'texture', 'Texture'), (13, 'particles', 'Particles'),
    (14, 'bone', 'Rig'), (15, 'paint_3d', 'Paint')]:
    alias(f'data_tab_{number:02}', source, label, 'Properties')


COMMAND_OVERRIDES = {
    'file.new': 'new_file', 'file.open': 'open_project',
    'file.save': 'save', 'file.save_as': 'save_as',
    'file.import_obj': 'import', 'file.export_obj': 'export_obj',
    'file.export_glb': 'export_glb', 'file.save_asset': 'save_asset',
    'help.documentation': 'help',
    'window.settings': 'settings', 'window.command_palette': 'command_palette',
    'window.reference_manager': 'reference_manager',
    'model.make_face': 'make_face', 'model.separate_selection': 'separate',
    'model.scale_selection': 'scale', 'model.weld': 'weld',
    'model.cut': 'boolean_cut', 'model.fuse': 'boolean_fuse',
    'model.intersect': 'boolean_intersect', 'model.join': 'boolean_join',
    'model.flip_normals': 'flip_normals',
    'model.push_pull': 'pushpull', 'model.loop_cut': 'loop_cut',
    'model.instantiate_asset': 'asset_library',
    'model.select_all': 'select_all', 'model.deselect_all': 'deselect_all',
    'model.invert_selection': 'invert_selection',
    'model.tool_cursor': 'cursor_3d', 'model.tool_lasso': 'select_lasso',
    'model.tool_transform': 'transform', 'model.measure': 'measure',
    'select.all': 'select_all', 'select.none': 'deselect_all',
    'select.invert': 'invert_selection', 'select.linked': 'select_linked',
    'select.cycle_domain': 'mode_edit',
    'select.domain_object': 'mode_object', 'select.domain_vertex': 'select_vertex',
    'select.domain_edge': 'select_edge', 'select.domain_face': 'select_face',
    'tools.toggle_proportional': 'proportional_editing',
    'tools.toggle_snap': 'snap_magnet',
    'paint.bake_decal': 'bake_decal',
    'paint.bake_reference': 'bake_reference',
    'paint.set_decal_transform': 'paint_decal',
    'uv.pack_islands': 'uv_pack', 'uv.project_reference': 'uv_project_reference',
    'uv.project_view': 'uv_project_view', 'uv.relax': 'uv_relax',
    'uv.stitch': 'uv_stitch', 'uv.unwrap_auto': 'uv_unwrap',
    'uv.unwrap': 'uv_unwrap',
    'view.toggle_projection': 'view_perspective',
    'view.toggle_wireframe': 'shading_wireframe',
    'view.toggle_wire_overlay': 'wire_overlay',
    'view.toggle_xray': 'xray', 'view.toggle_uv_checker': 'uv_checker',
    'view.toggle_face_orientation': 'flip_normals',
    'view.toggle_nav_hud': 'axis_gizmo',
    'view.reset_camera': 'camera',
    'view.back': 'view_back', 'view.bottom': 'view_bottom',
    'view.front': 'view_front', 'view.left': 'view_left',
    'view.right': 'view_right', 'view.top': 'view_top',
    'view.isometric_ne': 'view_perspective',
    'view.isometric_nw': 'view_perspective',
    'view.isometric_se': 'view_perspective',
    'view.isometric_sw': 'view_perspective',
    'view.cancel_active': 'close',
    'object.toggle_edit_pivot': 'pivot_median',
}


def redraw(key: str, label: str, outline: str, filled: str) -> None:
    """Substitui a arte de um ID que antes só reaproveitava outro ícone (ambiguidade)."""
    group, _, _, _, status = ICONS[key]
    ICONS[key] = group, label, outline, filled, status


HEX = 'M8.5 3h7L21 8.5v7L15.5 21h-7L3 15.5v-7z'
FACET = 'M3.5 9h17M3.5 15h17'
redraw('shade_flat', 'Shade Flat', ink(p(HEX), p(FACET)), solid(p(HEX)) + ink(p(FACET, stroke='#252735')))
SMOOTH = 'M7.5 9.5a6 6 0 0 1 4-3.5M8 16c2.5 2 6.5 1.5 8.5-2'
redraw('shade_smooth', 'Shade Smooth', ink(c(12, 12, 9), p(SMOOTH)), solid(c(12, 12, 9)) + ink(p(SMOOTH, stroke='#252735')))
CORNERS = 'M3 8V5a2 2 0 0 1 2-2h3M16 3h3a2 2 0 0 1 2 2v3M21 16v3a2 2 0 0 1-2 2h-3M8 21H5a2 2 0 0 1-2-2v-3'
redraw('isolate', 'Isolate', ink(r(7, 7, 10, 10, 2), p(CORNERS)), solid(r(6, 6, 12, 12, 2)) + ink(p(CORNERS)))
FACE = 'M4 17l8-4 8 4-8 4z'
ARROW = 'M12 13V3m-3.5 3.5L12 3l3.5 3.5'
redraw('normals_overlay', 'Normals Overlay', ink(p(FACE), p(ARROW)), solid(p(FACE)) + ink(p(ARROW)))
FLIP = 'M8 20V9m-3 3 3-3 3 3M16 4v11m-3-3 3 3 3-3'
redraw('flip_normals', 'Flip Normals', ink(p(FLIP)), ink(p(FLIP, stroke_width='2.5')))
BACK = 'M12 12l9-5M12 12 3 7M12 12v10'
redraw('backface_culling', 'Backface Culling', ink(p(CUBE), p(BACK, stroke_dasharray='2 2.4')), solid(p('M12 12l9-5v10l-9 5z')) + ink(p(CUBE)))
redraw('bounding_box', 'Bounding Box', ink(r(4, 4, 16, 16, 1, stroke_dasharray='3 2.4')) + solid(c(4, 4, 1.8), c(20, 4, 1.8), c(4, 20, 1.8), c(20, 20, 1.8)), solid(r(7, 7, 10, 10, 1)) + ink(r(3, 3, 18, 18, 1, stroke_dasharray='3 2.4')) + solid(c(3, 3, 1.8), c(21, 3, 1.8), c(3, 21, 1.8), c(21, 21, 1.8)))
MAX = 'M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5'
redraw('maximize', 'Maximize', ink(p(MAX)), solid(r(7, 7, 10, 10, 1)) + ink(p(MAX)))
redraw('minimize', 'Minimize', ink(p('M6 18h12')), ink(p('M6 18h12'), ) + solid(r(6, 15, 12, 4, 1)))
AXES = 'M12 3v18M3 12h18'
redraw('origin', 'Origin', ink(p(AXES)) + solid(c(12, 12, 3)), solid(c(12, 12, 5)) + ink(p(AXES)))
redraw('selection_center', 'Selection Center', ink(r(3, 3, 18, 18, 2, stroke_dasharray='3 2.4')) + solid(c(12, 12, 2.6)), solid(r(3, 3, 18, 18, 2)) + solid(c(12, 12, 3.4, fill='#252735')))
redraw('paint', 'Paint Roller', ink(r(4, 3, 14, 6, 1.5), p('M18 6h2v5h-9v3'), r(9, 14, 4, 7, 1)), solid(r(4, 3, 14, 6, 1.5), r(9, 14, 4, 7, 1)) + ink(p('M18 6h2v5h-9v3')))
REF = 'M9 14l3-3 3 3 2-2 2 2'
redraw('reference_manager', 'Reference Manager', ink(r(6, 3, 15, 13, 2), p('M3 8v11a2 2 0 0 0 2 2h11'), c(11, 8, 1.4), p(REF)), solid(r(6, 3, 15, 13, 2)) + ink(p('M3 8v11a2 2 0 0 0 2 2h11')) + ink(p(REF, stroke='#252735')))
redraw('object_mesh', 'Object Mesh', ink(p(CUBE_EDGE), p('M3 7v10l9 5 9-5V7')) + solid(c(12, 2, 1.6), c(21, 7, 1.6), c(21, 17, 1.6), c(12, 22, 1.6), c(3, 17, 1.6), c(3, 7, 1.6), c(12, 12, 1.6)), solid(p(CUBE)) + solid(c(12, 12, 1.8, fill='#252735')))


def command_map() -> dict[str, str]:
    repo = ROOT.parent.parent
    commands = set(re.findall(r'\| `(\w+\.\w+)` \|', (repo / 'docs/generated/COMMANDS.md').read_text()))
    commands |= set(re.findall(r'command-executed\("([\w.]+)"\)', (repo / 'crates/ui-slint/ui/app.slint').read_text()))
    result = {}
    for command in sorted(commands):
        prefix, name = command.split('.', 1)
        icon = COMMAND_OVERRIDES.get(command, name[4:] if prefix == 'model' and name.startswith('add_') else name)
        if icon not in ICONS:
            raise ValueError(f'Unmapped command {command} => {icon}')
        result[command] = icon
    return result


KNOCKOUT = '#252735'
SHAPES = {'path', 'circle', 'rect', 'ellipse', 'line', 'polygon', 'polyline'}


def _tag(el: ET.Element) -> str:
    return el.tag.split('}')[-1]


def _open(el: ET.Element, **override: str | None) -> str:
    attrs = dict(el.attrib)
    for key, value in override.items():
        key = key.replace('_', '-')
        if value is None:
            attrs.pop(key, None)
        else:
            attrs[key] = value
    return '<' + _tag(el) + ''.join(f' {k}="{html.escape(v, quote=True)}"' for k, v in attrs.items())


def _wrap(chain: list[ET.Element], leaf: str) -> str:
    for group in reversed(chain):
        leaf = _open(group) + '>' + leaf + '</g>'
    return leaf


def themeable(geometry: str) -> str:
    """Turn background-coloured "knockout" strokes/fills into real SVG masks.

    Icons are authored with `#252735` marks that fake a cut-out on a dark panel. That
    only works on one background. Here every knockout becomes transparent in a mask
    applied to everything drawn before it, so the art is monochrome `currentColor`
    with real holes and can be tinted by any theme (Slint `colorize`).
    """
    root = ET.fromstring(f'<svg xmlns="http://www.w3.org/2000/svg">{geometry}</svg>')
    content: list[str] = []
    pending: list[str] = []
    counter = 0

    def flush() -> None:
        nonlocal content, pending, counter
        if not pending:
            return
        counter += 1
        mask = (f'<mask id="k{counter}" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24">'
                '<rect width="24" height="24" fill="#fff"/>' + ''.join(pending) + '</mask>')
        content = [mask, f'<g mask="url(#k{counter})">' + ''.join(content) + '</g>']
        pending = []

    def walk(el: ET.Element, chain: list[ET.Element]) -> None:
        for child in el:
            if _tag(child) == 'g':
                walk(child, chain + [child])
                continue
            if _tag(child) not in SHAPES:
                content.append(ET.tostring(child, encoding='unicode'))
                continue
            fill, stroke = child.get('fill'), child.get('stroke')
            if fill == KNOCKOUT or stroke == KNOCKOUT:
                if fill == KNOCKOUT:
                    pending.append(_wrap(chain, _open(child, fill='#000', stroke='none') + '/>'))
                    if stroke not in (None, 'none', KNOCKOUT):
                        flush()
                        content.append(_wrap(chain, _open(child, fill='none') + '/>'))
                else:
                    keep_fill = None if fill is None else fill
                    pending.append(_wrap(chain, _open(child, stroke='#000', fill=keep_fill or 'none') + '/>'))
            else:
                flush()
                content.append(_wrap(chain, _open(child) + '/>'))

    walk(root, [])
    flush()
    return ''.join(content)


def svg(label: str, geometry: str) -> str:
    safe_label = html.escape(label)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" '
            f'width="24" height="24" role="img" '
            f'aria-label="{safe_label}"><title>{safe_label}</title>'
            f'{themeable(geometry)}</svg>\n')


def slug(group: str) -> str:
    return re.sub(r'[^a-z0-9]+', '-', group.lower()).strip('-')


def gallery(catalog: list[dict[str, str]]) -> str:
    cards = []
    for entry in catalog:
        key, group, label, status = [entry[k] for k in ('id', 'group', 'label', 'status')]
        src = '../petunia-outline/svg/' + slug(group) + '/' + key + '.svg'
        cards.append(f'<article class="card" data-group="{html.escape(group)}" data-search="{html.escape((key+" "+label).lower())}">'
                     f'<div class="tile"><img loading="lazy" src="{src}" data-outline="{src}" data-filled="{src.replace("petunia-outline", "petunia-filled")}" alt="{html.escape(label)}"></div>'
                     f'<div class="name">{html.escape(label)}</div><code>{key}</code>'
                     f'{"<small>Roadmap</small>" if status == "planned" else ""}</article>')
    groups = sorted({entry['group'] for entry in catalog})
    options = ''.join(f'<option value="{html.escape(g)}">{html.escape(g)}</option>' for g in groups)
    return '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Petunia3D Icon Gallery</title><style>
:root{font:14px/1.4 system-ui,sans-serif;color:#e8ebf2;background:#141720}*{box-sizing:border-box}body{margin:0 auto;max-width:1500px;padding:32px}
header{display:flex;align-items:end;justify-content:space-between;gap:24px;flex-wrap:wrap;margin-bottom:28px}h1{font-size:27px;margin:0 0 5px}p{color:#aeb7cc;margin:0}
.controls{display:flex;gap:10px;flex-wrap:wrap}input,select,button{background:#242836;border:1px solid #485065;border-radius:8px;color:#e8ebf2;padding:9px 12px;font:inherit}
input{width:245px}button{cursor:pointer}button[aria-pressed=true]{border-color:#b58cff;background:#44345f}
main{display:grid;grid-template-columns:repeat(auto-fill,minmax(138px,1fr));gap:12px}.card{background:#1c202b;border:1px solid #343b4e;border-radius:10px;padding:12px;min-height:127px}
.tile{height:49px;display:flex;align-items:center}.tile img{width:32px;height:32px;filter:invert(1) brightness(.92)}.name{font-weight:600;margin-bottom:4px}code{font:11px ui-monospace,monospace;color:#aeb7cc;overflow-wrap:anywhere}small{display:block;color:#b58cff;margin-top:4px}
.card[hidden]{display:none}footer{color:#aeb7cc;margin-top:25px}
</style><header><div><h1>Petunia3D · Icon System</h1><p>Original 24 × 24 SVGs · transparent canvas · MODEL / PAINT / UV and editor actions · <span id="count"></span> visible</p></div>
<div class="controls"><input id="search" type="search" placeholder="Search icon or ID" aria-label="Search icons"><select id="group" aria-label="Category"><option value="">All categories</option>''' + options + '''</select>
<button id="outline" aria-pressed="true">Outline</button><button id="filled" aria-pressed="false">Filled</button></div></header><main>''' + ''.join(cards) + '''</main>
<footer>Roadmap concepts are marked. An icon does not imply feature completion. Artwork is original; no third-party icon files were copied.</footer>
<script>const cards=[...document.querySelectorAll('.card')],q=document.querySelector('#search'),g=document.querySelector('#group');let style='outline';
function update(){let n=0;for(const card of cards){const ok=card.dataset.search.includes(q.value.toLowerCase())&&(!g.value||card.dataset.group===g.value);card.hidden=!ok;if(ok)n++;card.querySelector('img').src=card.querySelector('img').dataset[style]}document.querySelector('#count').textContent=n}
for(const id of ['outline','filled'])document.querySelector('#'+id).addEventListener('click',()=>{style=id;for(const button of document.querySelectorAll('button'))button.setAttribute('aria-pressed',button.id===id);update()});q.addEventListener('input',update);g.addEventListener('change',update);update();</script></html>'''


def main() -> None:
    mapping = command_map()
    repo = ROOT.parent.parent
    legacy = (repo / 'crates/ui/src/icon_registry.rs').read_text()
    ids = set(re.findall(r'=> "([a-z0-9_]+)"\.into\(\)', legacy[:legacy.index('pub fn all()')]))
    ids |= set(re.findall(r'Custom\("([a-z0-9_]+)"\)', legacy[:legacy.index('pub fn phosphor_glyph')]))
    slint = (repo / 'crates/ui-slint/ui/app.slint').read_text()
    ids |= {name.replace('-', '_') for name in re.findall(r'@image-url\("icons/([^"/]+)\.svg"', slint) if name != 'logo'}
    ids |= set(tomllib.loads((repo / 'assets/tools.toml').read_text())['tools'])
    missing = ids - ICONS.keys()
    if missing:
        raise ValueError(f'Missing UI/toolbar icon IDs: {sorted(missing)}')
    same = [key for key, (_, _, outline, filled, _) in ICONS.items() if outline == filled]
    if same:
        raise ValueError(f'Identical variant markup: {same}')
    catalog = []
    for variant in ('outline', 'filled'):
        pack = ROOT / ('petunia-' + variant)
        pack.mkdir(parents=True, exist_ok=True)
        shutil.rmtree(pack / 'svg', ignore_errors=True)
        (pack / 'manifest.toml').write_text(
            f'[icon_pack]\nid = "petunia-{variant}"\nname = "Petunia {variant.title()}"\n'
            'version = "1.0.0"\nauthor = "Petunia3D Team"\nlicense = "MIT"\n'
            f'description = "Original Petunia3D {variant} SVG icon family, 24px grid"\n')
        lines = ['# Semantic IconId => relative SVG asset. Generated by ../petunia-dual/generate.py', '[icons]']
        for key, (group, label, outline, filled, status) in sorted(ICONS.items()):
            rel = f'svg/{slug(group)}/{key}.svg'
            path = pack / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            content = svg(label, outline if variant == 'outline' else filled)
            ET.fromstring(content)
            path.write_text(content)
            lines.append(f'{key} = "{rel}"')
            if variant == 'outline':
                catalog.append({'id': key, 'group': group, 'label': label, 'status': status})
        (pack / 'icons.toml').write_text('\n'.join(lines) + '\n')
    (SOURCE / 'catalog.json').write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + '\n')
    (SOURCE / 'command-map.json').write_text(json.dumps(mapping, indent=2, ensure_ascii=False) + '\n')
    (SOURCE / 'gallery.html').write_text(gallery(catalog))
    print(f'{len(ICONS)} semantic IDs × 2 variants; {len(mapping)} commands mapped')


if __name__ == '__main__':
    main()
