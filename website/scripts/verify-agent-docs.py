#!/usr/bin/env python3
"""Dependency-free integrity check for Petunia3D agent-facing website documentation.

Checks current repository files and the pinned Prumo inventory. This does NOT
fetch the internet or claim that the upstream Prumo main branch is unchanged.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

SITE = Path(__file__).resolve().parents[1]
DOCS = SITE / "docs"
ROOT = SITE.parent
PREFIX = "src/prumo/resources/workforce"
BASE = "https://github.com/poppy-lat/prumo/blob/main/"
RULES = {"agents": "AGENT.md", "skills": "SKILL.md", "recipes": "RECIPE.md"}
REPORT: list[str] = []


def fail(message: str) -> None:
    REPORT.append(message)


def internal_links(markdown: str) -> list[str]:
    return re.findall(r"(?<!!)\[[^\]]+\]\(([^)\s]+)(?:\s+[^)]*)?\)", markdown)


def resolve_link(path: Path, link: str) -> Path | None:
    if link.startswith("#") or link.startswith("//"):
        return None
    parsed = urlsplit(link)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return None
    # Do not follow untrusted paths outside the docs root.
    dest = (path.parent / unquote(parsed.path)).resolve()
    if not dest.is_relative_to(DOCS.resolve()):
        fail(f"Link escapes docs/: {path.relative_to(ROOT)} -> {link}")
        return None
    return dest


def check() -> None:
    manifest = json.loads((DOCS / "manifest.json").read_text(encoding="utf-8"))
    domains = manifest.get("domains", [])
    ids = [d["id"] for d in domains]
    if len(ids) != len(set(ids)):
        fail("Duplicate domain ids in docs/manifest.json")

    routes: set[str] = {"home.md", "about.md"}
    for domain in domains:
        for entry in domain["files"]:
            route = entry["path"]
            if route in routes:
                fail(f"Duplicate route: {route}")
            routes.add(route)
            file = (DOCS / route).resolve()
            if not file.is_relative_to(DOCS.resolve()) or not file.is_file():
                fail(f"Missing or unsafe route: {route}")

    group = next((d for d in domains if d["id"] == "code-agents"), None)
    if not group:
        fail("Missing code-agents domain")
        return
    agent_pages = [DOCS / e["path"] for e in group["files"]]
    if len(agent_pages) != 15:
        fail(f"Expected 15 agent pages, got {len(agent_pages)}")

    catalog_path = DOCS / "16-code-agents" / "workforce-catalog.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    counts = {"agents": 39, "skills": 189, "recipes": 20}
    if catalog.get("counts") != counts:
        fail(f"Unexpected source inventory counts {catalog.get('counts')}")

    seen: dict[str, set[str]] = {}
    for category, file_name in RULES.items():
        entries = catalog.get(category, [])
        ids_set: set[str] = set()
        seen[category] = ids_set
        for item in entries:
            key = item.get("id", "")
            if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", key):
                fail(f"Invalid catalog id: {category}/{key}")
            if key in ids_set:
                fail(f"Duplicate catalog id: {category}/{key}")
            ids_set.add(key)
            wanted = f"{PREFIX}/{category}/{key}/{file_name}"
            if item.get("source_file") != wanted:
                fail(f"Incorrect source file for {category}/{key}")
            if item.get("source_url") != BASE + wanted:
                fail(f"Incorrect direct link for {category}/{key}")
            man_name = "recipe.json" if category == "recipes" else "manifest.json"
            manifest_url = BASE + f"{PREFIX}/{category}/{key}/{man_name}"
            if item.get("manifest_url") != manifest_url:
                fail(f"Incorrect manifest link: {category}/{key}")
        if len(entries) != counts[category]:
            fail(f"Count drift in {category}: {len(entries)} instead of {counts[category]}")

    catalog_page_files = {
        "agents": [DOCS / "16-code-agents" / "agents-catalog.md"],
        "skills": list((DOCS / "16-code-agents").glob("skills-*.md")),
        "recipes": [DOCS / "16-code-agents" / "recipes-catalog.md"],
    }
    for category, file_name in RULES.items():
        expected = seen[category]
        referenced: list[str] = []
        prefix = BASE + PREFIX + f"/{category}/"
        for page in catalog_page_files[category]:
            for link in internal_links(page.read_text(encoding="utf-8")):
                if not link.startswith(prefix) or not link.endswith("/" + file_name):
                    continue
                sub = link[len(prefix):].split("/")
                if len(sub) == 2:
                    referenced.append(sub[0])
        if set(referenced) != expected:
            fail(f"Catalog Markdown incomplete for {category}: missing={sorted(expected-set(referenced))}; extras={sorted(set(referenced)-expected)}")
        if len(referenced) != len(expected):
            fail(f"Catalog Markdown duplicates for {category}: {len(referenced)} links vs {len(expected)} unique")

    for page in agent_pages:
        md = page.read_text(encoding="utf-8")
        for href in internal_links(md):
            dest = resolve_link(page, href)
            if dest is None:
                continue
            if not dest.is_file():
                fail(f"Broken relative link: {page.relative_to(DOCS)} -> {href}")
            elif dest.suffix == ".md" and dest.relative_to(DOCS).as_posix() not in routes:
                fail(f"Markdown page not listed in manifest: {dest.relative_to(DOCS)}")

    if not (SITE / "llms.txt").is_file():
        fail("website/llms.txt missing")
    if not (ROOT / "AGENTS.md").is_file():
        fail("Root AGENTS.md missing")

    if not REPORT:
        print(f"PASS: {len(routes)} site routes, 15 agent pages, "
              f"{len(seen['agents'])} agents / {len(seen['skills'])} skills / "
              f"{len(seen['recipes'])} recipes, links/catalog consistent")


if __name__ == "__main__":
    try:
        check()
    except (OSError, KeyError, ValueError, TypeError) as exc:
        fail(f"Verification could not complete: {exc}")
    if REPORT:
        print("FAIL: Agent documentation integrity")
        for item in REPORT:
            print(f"  - {item}")
        sys.exit(1)
