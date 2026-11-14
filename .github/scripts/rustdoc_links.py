#!/usr/bin/env python3
"""mdBook preprocessor that makes rustdoc links work on the docs site.

Most guides in `docs/` are also included in the crates' API documentation, so
they link to items the way rustdoc expects: intra-doc paths such as
`crate::asset` or `Router::builder`, shortcut links such as [`Router`], and
URLs relative to the item's page such as `struct.Router.html`. mdBook cannot
resolve any of these.

Rustdoc already resolved them when it rendered the same Markdown, so this
script reads the answers from `target/doc/topcoat`:

1. Find the rustdoc page that includes each chapter.
2. Resolve relative `.html` links against that page's URL.
3. Resolve intra-doc links by looking up the link text in that page.

A link whose target is the page of another chapter points to that chapter.
Every other link points to docs.rs.

Build the API docs before the book:

    cargo doc -p topcoat --all-features --no-deps
    mdbook build

Without them the book still builds, with these links left as they are. In CI
missing API docs are an error.
"""

import html
import json
import os
import posixpath
import re
import sys
from pathlib import Path
from urllib.parse import urljoin

CRATE = "topcoat"
DOCS_RS = "https://docs.rs/{package}/latest/{rest}"

FENCE = re.compile(r"^\s*(```|~~~)")
DEFINITION = re.compile(r"^(\s{0,3}\[)((?:`[^`]*`|[^\[\]`])+)(\]:\s*)(\S+)(.*)$")
TEXT = r"(?:`[^`\n]*`|[^\[\]`\n])+"
INLINE = re.compile(
    r"(?P<code>`+[^`\n]*`+)"
    r"|(?P<image>!\[)"
    rf"|\[(?P<text>{TEXT})\](?:\((?P<dest>[^()\s]*)\)|\[(?P<label>(?:{TEXT})?)\])?"
)
ANCHOR = re.compile(r'<a\b([^>]*)\bhref="([^"]*)"([^>]*)>(.*?)</a>', re.S)
HEADING = re.compile(r"^#{1,6}\s+(.*?)\s*$")
HTML_DEST = re.compile(r"^[^\s:]*\.html(#.*)?$")
DISAMBIGUATOR = re.compile(r"^[a-z]+@")
EXTERNAL = re.compile(r"^(https?:|mailto:)")


def warn(message):
    print(f"rustdoc-links: {message}", file=sys.stderr)


def plain(text, rendered=False):
    """Link text as rustdoc displays it, for comparing Markdown with HTML."""
    if rendered:
        text = html.unescape(re.sub(r"<[^>]+>", "", text))
        # Rustdoc renders straight quotes as curly ones.
        text = text.translate(str.maketrans("\u2018\u2019\u201c\u201d", "''\"\""))
    text = text.replace("`", "").replace("\\", "")
    return " ".join(text.split())


def external(dest):
    """Whether mdBook can already follow `dest`."""
    return bool(EXTERNAL.match(dest)) or dest.startswith("#") or ".md" in dest


class Page:
    """A rendered rustdoc page and the links in its main documentation."""

    def __init__(self, root, rel):
        self.rel = rel
        source = (root / rel).read_text(encoding="utf8")
        start = source.find("top-doc")
        end = source.find('class="section-header"', start)
        doc = source[start : end if end != -1 else len(source)] if start != -1 else ""
        self.titles = {plain(t, rendered=True) for t in re.findall(r"<h[2-6][^>]*>(.*?)</h[2-6]>", doc, re.S)}
        self.titles = {t.strip("\u00a7 ") for t in self.titles}
        self.links = {}
        for before, href, after, text in ANCHOR.findall(doc):
            if "doc-anchor" in before + after or href.startswith("#"):
                continue
            self.links.setdefault(plain(text, rendered=True), []).append(href)
        self.used = {}

    def lookup(self, text):
        """The target rustdoc chose for a link with this text."""
        hrefs = self.links.get(plain(text))
        if not hrefs:
            return None
        if len(set(hrefs)) == 1:
            return hrefs[0]
        # The same text links to different items. Follow the document order.
        index = self.used.get(plain(text), 0)
        self.used[plain(text)] = index + 1
        return hrefs[min(index, len(hrefs) - 1)]


class Resolver:
    def __init__(self, doc_root, chapters):
        self.root = doc_root
        self.cache = {}
        # Chapter path -> rustdoc page, and the reverse.
        self.pages = {}
        for path, content in chapters.items():
            page = self.find_page(path, content)
            if page:
                self.pages[path] = page
        self.chapters = {page.rel: path for path, page in self.pages.items()}

    def page(self, rel):
        if rel not in self.cache:
            self.cache[rel] = Page(self.root, rel)
        return self.cache[rel]

    def find_page(self, path, content):
        """Find the rustdoc page that includes the chapter at `path`.

        `docs/` follows the module structure, so `router/page.md` documents
        either the module `router::page` or an item in `router`. The item does
        not always share the file's name, so compare section headings too.
        """
        stem = posixpath.splitext(path)[0]
        parent, name = posixpath.split(stem)
        directory = self.root / CRATE / parent
        if not directory.is_dir():
            return None
        headings = set()
        fenced = False
        for line in content.splitlines():
            if FENCE.match(line):
                fenced = not fenced
            elif not fenced and (match := HEADING.match(line)):
                headings.add(plain(match.group(1)))

        candidates = [p for p in directory.glob("*.html") if "!" not in p.name]
        candidates += directory.glob("*/index.html")
        best, best_score = None, (0, 0)
        for candidate in candidates:
            item = candidate.parent.name if candidate.name == "index.html" else candidate.name.split(".")[1]
            named = item.lower() == name.lower()
            if not named and not headings:
                continue
            page = self.page(candidate.relative_to(self.root).as_posix())
            matched = len(headings & page.titles)
            if headings and matched < len(headings):
                continue
            score = (matched, int(named))
            if score > best_score:
                best, best_score = page, score
        return best

    def by_path(self, dest):
        """Resolve `crate::module::item` from the files rustdoc generated."""
        dest, _, fragment = dest.partition("#")
        parts = DISAMBIGUATOR.sub("", dest).split("::")
        if parts[0] != "crate" or len(parts) < 2:
            return None
        directory = Path(CRATE)
        for part in parts[1:-1]:
            directory /= part
        last = parts[-1].rstrip("!")
        if (self.root / directory / last / "index.html").is_file():
            rel = directory / last / "index.html"
        else:
            found = [p for p in (self.root / directory).glob(f"*.{last}.html") if "!" not in p.name]
            if len(found) != 1:
                return None
            rel = directory / found[0].name
        return rel.as_posix() + (f"#{fragment}" if fragment else "")

    def url(self, chapter, base, href):
        """Turn a link from the rustdoc page `base` into a link for the book."""
        if EXTERNAL.match(href):
            return href
        if "::" in href:
            # Rustdoc could not resolve the link either.
            return None
        target = urljoin(f"https://docs.rs/{base}", href)
        rel, _, fragment = target.removeprefix("https://docs.rs/").partition("#")
        # Stay in the book when the target is the page of another chapter.
        # Item anchors such as `#method.new` only exist in the API docs.
        if rel in self.chapters and "." not in fragment:
            other = posixpath.relpath(self.chapters[rel], posixpath.dirname(chapter) or ".")
            return other + (f"#{fragment}" if fragment else "")
        package = rel.split("/", 1)[0].replace("_", "-")
        return DOCS_RS.format(package=package, rest=rel) + (f"#{fragment}" if fragment else "")

    def resolve(self, chapter, text, dest):
        """Return the book link for `dest`, or `None` to leave it alone."""
        page = self.pages.get(chapter)
        if dest is not None and external(dest):
            return None
        if dest is not None and HTML_DEST.match(dest):
            return self.url(chapter, page.rel, dest) if page else None
        if page and (href := page.lookup(text)):
            return self.url(chapter, page.rel, href)
        if dest and (rel := self.by_path(dest)):
            return self.url(chapter, f"{CRATE}/index.html", f"../{rel}")
        return None


def rewrite(resolver, chapter, content, unresolved):
    lines = content.split("\n")
    definitions = {}
    fenced = False
    prose = []
    for number, line in enumerate(lines):
        if FENCE.match(line):
            fenced = not fenced
        elif not fenced:
            prose.append(number)
            if match := DEFINITION.match(line):
                definitions[plain(match.group(2))] = match.group(4)

    def replace(match):
        if match.group("code") or match.group("image"):
            return match.group(0)
        text, dest, label = match.group("text"), match.group("dest"), match.group("label")
        explicit = dest is not None
        if not explicit:
            # `[text][label]` or `[text]`: use the definition, else the label
            # is an intra-doc path.
            key = plain(label or text)
            dest = definitions.get(key)
            if dest is not None and (external(dest) or HTML_DEST.match(dest)):
                return match.group(0)
        url = resolver.resolve(chapter, text, dest)
        if url:
            return f"[{text}]({url})"
        if explicit and not external(dest):
            unresolved.append(f"{chapter}: [{plain(text)}]({dest})")
        return match.group(0)

    for number in prose:
        line = lines[number]
        if match := DEFINITION.match(line):
            url = resolver.resolve(chapter, match.group(2), match.group(4))
            if url:
                lines[number] = f"{match.group(1)}{match.group(2)}{match.group(3)}{url}{match.group(5)}"
            continue
        lines[number] = INLINE.sub(replace, line)
    return "\n".join(lines)


def walk(items):
    for item in items:
        chapter = item.get("Chapter") if isinstance(item, dict) else None
        if chapter:
            yield chapter
            yield from walk(chapter.get("sub_items", []))


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "supports":
        return 0
    context, book = json.load(sys.stdin)
    target = os.environ.get("CARGO_TARGET_DIR") or os.path.join(context["root"], "target")
    doc_root = Path(target) / "doc"
    if not (doc_root / CRATE / "index.html").is_file():
        message = f"no API docs in {doc_root}; run `cargo doc -p {CRATE} --all-features --no-deps` first"
        if os.environ.get("CI"):
            warn(f"error: {message}")
            return 1
        warn(f"{message}. Leaving rustdoc links unresolved.")
        json.dump(book, sys.stdout)
        return 0

    chapters = [c for c in walk(book.get("items") or book.get("sections") or []) if c.get("path")]
    resolver = Resolver(doc_root, {c["path"]: c["content"] for c in chapters})
    unresolved = []
    for chapter in chapters:
        chapter["content"] = rewrite(resolver, chapter["path"], chapter["content"], unresolved)
    for link in unresolved:
        warn(f"unresolved link in {link}")
    json.dump(book, sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
