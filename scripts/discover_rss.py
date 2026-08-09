#!/usr/bin/env python3
"""Discover direct RSS/Atom feed URLs from a markdown document of homepage links.

Usage:
    python3 scripts/discover_rss.py <markdown_file> [--out report.txt]

For each unique http(s) URL in the markdown it:
  1. Fetches the page (browser User-Agent, redirects followed, timeouts).
  2. Auto-detects a feed via <link rel="alternate" type="application/rss+xml|atom+xml">
  3. Falls back to well-known platform patterns when auto-detect finds nothing.

Emits a report where each feed maps to its resolved feed URL or "NEEDS MANUAL".

Dependencies: requests, beautifulsoup4  (present in repo system python3).
"""

import argparse
import re
import sys
from urllib.parse import urljoin, urlparse, urlunparse

import requests
from bs4 import BeautifulSoup

UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/122.0 Safari/537.36"
)
TIMEOUT = 20
FEED_MIMES = {"application/rss+xml", "application/atom+xml", "application/feed+json"}

URL_RE = re.compile(r"https?://[^\s<>\"'`)\]\|]+")


def extract_urls(markdown: str) -> list[str]:
    urls = []
    for m in URL_RE.finditer(markdown):
        url = m.group(0).rstrip(".,;:!?)]}")
        if url not in urls:
            urls.append(url)
    return urls


def get(url: str) -> requests.Response | None:
    try:
        return requests.get(url, headers={"User-Agent": UA}, timeout=TIMEOUT, allow_redirects=True)
    except requests.RequestException:
        return None


def norm_host(host: str) -> str:
    host = host.lower().removeprefix("www.").removeprefix("blog.")
    return host


def platform_fallback(url: str) -> str | None:
    """Best-effort RSS url for known platforms that don't expose a <link> favicon."""

    p = urlparse(url)
    host = norm_host(p.netloc)
    path = p.path.rstrip("/")

    # arXiv listing pages
    m = re.match(r"^/list/([A-Za-z.]+)/recent$", path or "/")
    if m and host == "arxiv.org":
        cat = m.group(1)
        return f"https://export.arxiv.org/rss/{cat}"

    # Reddit subreddits
    if host == "reddit.com":
        return f"https://www.reddit.com{path}.rss"

    # Lobsters
    if host == "lobste.rs":
        return "https://lobste.rs/rss"

    # Blogspot / blogspot.com
    if host.endswith("blogspot.com"):
        return f"https://{p.netloc}/feeds/posts/default?alt=rss"

    # WordPress / generic
    if host.endswith("wordpress.com") or ".wordpress." in host:
        return f"https://{p.netloc}/feed/"

    return None


def discover(url: str) -> tuple[str | None, str]:
    """Returns (feed_url, note)."""
    resp = get(url)
    if resp is None:
        return None, "FETCH FAILED"
    if resp.status_code == 404:
        return None, f"HTTP 404"
    if resp.status_code >= 400:
        return None, f"HTTP {resp.status_code}"

    ctype = resp.headers.get("Content-Type", "")
    # If we've already fetched a feed, short-circuit.
    if "xml" in ctype or "json" in ctype:
        # confirm it's actually a feed-like doc
        text = resp.text[:500].lower()
        if "<rss" in text or "<feed" in text or "rss:" in text:
            return resp.url, "direct feed"

    soup = BeautifulSoup(resp.text, "html.parser")
    for link in soup.find_all("link", rel="alternate"):
        t = (link.get("type") or "").lower()
        href = link.get("href") or ""
        if t in FEED_MIMES and href and not href.startswith("#"):
            return urljoin(resp.url, href), "auto <link>"

    # Content-Type based
    fb = platform_fallback(url)
    if fb:
        return fb, "platform fallback"

    return None, "NEEDS MANUAL"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("markdown")
    ap.add_argument("--out", default="")
    args = ap.parse_args()

    with open(args.markdown, encoding="utf-8") as f:
        md = f.read()

    urls = extract_urls(md)
    print(f"# {len(urls)} unique URLs to inspect\n", flush=True)

    results = []
    for url in urls:
        feed, note = discover(url)
        results.append((url, feed, note))
        status = feed or note
        print(f"{url}\n  -> {status}   [{note}]", flush=True)

    if args.out:
        with open(args.out, "w", encoding="utf-8") as f:
            for url, feed, note in results:
                f.write(f"{url}\t{feed or 'NEEDS_MANUAL'}\t{note}\n")
        print(f"\nWrote report to {args.out}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())