#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The corpus builder of the plugin survey. It lists the plugins of Spigot (through Spiget),
# Modrinth and Hangar that target Minecraft 1.20.1, most downloaded first, downloads the newest
# matching file of each, drops duplicates, and writes the index corpus.json. Every step appends
# its decisions to <dir>/work/<source>.jsonl, so a run that stops resumes where it stopped.
#
#     python3 -I corpus.py <dir> live
#     python3 -I corpus.py <dir> spigot [kept]        250 by default
#     python3 -I corpus.py <dir> modrinth [kept]      150 by default
#     python3 -I corpus.py <dir> hangar [kept]        100 by default
#     python3 -I corpus.py <dir> hangar_ext           the external Hangar links that name a file
#     python3 -I corpus.py <dir> redup                re-read every kept jar and drop duplicates
#     python3 -I corpus.py <dir> index                write <dir>/corpus.json
#
# The jars are untrusted data. They are streamed to <dir>/jars/ under names chosen here and read
# in memory with zipfile only. Nothing in a jar is executed, extracted, loaded or used as a path.
import hashlib
import io
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import zipfile
from urllib.parse import quote, urljoin, urlsplit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import plugin_yaml  # noqa: E402

BASE = JARS = WORK = INDEX = None
MAX_BYTES = 60 * 1024 * 1024
BUDGET = 3 * 1024 ** 3 - 200 * 1024 ** 2  # leave headroom under 3 GB
UA = "golem-plugin-survey/0.1 (compatibility research; github.com/skytekx)"
MIN_INTERVAL = 0.30  # seconds between requests to one host (< 4 per second)
RETRIES = 3
LOADERS = ["bukkit", "spigot", "paper", "purpur"]
REDIRECTS = (301, 302, 303, 307, 308)
_last = {}


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    """Hands every redirect back, so each hop is paced and its host seen."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


OPENER = urllib.request.build_opener(_NoRedirect)


def set_dir(path):
    global BASE, JARS, WORK, INDEX
    BASE = os.path.abspath(path)
    JARS = os.path.join(BASE, "jars")
    WORK = os.path.join(BASE, "work")
    INDEX = os.path.join(BASE, "corpus.json")


def pace(host):
    now = time.monotonic()
    wait = _last.get(host, 0) + MIN_INTERVAL - now
    if wait > 0:
        time.sleep(wait)
    _last[host] = time.monotonic()


def get(url, max_redirects=6, timeout=60):
    """GET with per-host pacing and redirects followed by hand. Returns (status, response, url)."""
    for _ in range(max_redirects + 1):
        host = urlsplit(url).hostname or ""
        pace(host)
        request = urllib.request.Request(url, headers={"User-Agent": UA})
        try:
            response = OPENER.open(request, timeout=timeout)
        except urllib.error.HTTPError as e:
            if e.code in REDIRECTS and e.headers.get("location"):
                location = e.headers["location"]
                e.close()
                url = urljoin(url, location)
                continue
            return e.code, e, url
        return response.status, response, url
    raise RuntimeError("too many redirects")


def get_json(url, tries=RETRIES):
    err = None
    for i in range(tries):
        try:
            status, r, _ = get(url)
            with r:
                if status == 200:
                    return json.loads(r.read())
            if status == 404:
                return None
            err = "HTTP %d" % status
            if status == 429:
                time.sleep(10 * (i + 1))
        except Exception as e:  # network errors
            err = repr(e)
        time.sleep(2 * (i + 1))
    raise RuntimeError("json fetch failed %s: %s" % (url, err))


def safe(s):
    return re.sub(r"[^A-Za-z0-9_.-]", "_", str(s))[:80]


def dir_bytes():
    total = 0
    for root, _, files in os.walk(BASE):
        for f in files:
            try:
                total += os.lstat(os.path.join(root, f)).st_size
            except OSError:
                pass
    return total


def download(url, tag):
    """Stream url to JARS/<tag>.part. Returns (path, sha256, sha1, bytes, final_url) or raises."""
    part = os.path.join(JARS, ".part-" + safe(tag))
    err = None
    for i in range(RETRIES):
        try:
            status, r, final = get(url, timeout=120)
            if status != 200:
                err = "HTTP %d" % status
                r.close()
                if status in (403, 404, 410):
                    break
                time.sleep(2 * (i + 1))
                continue
            with r:
                cl = r.headers.get("content-length")
                if cl and cl.isdigit() and int(cl) > MAX_BYTES:
                    raise TooBig(int(cl))
                h256, h1, n = hashlib.sha256(), hashlib.sha1(), 0
                with open(part, "wb") as f:
                    while True:
                        chunk = r.read(1 << 16)
                        if not chunk:
                            break
                        n += len(chunk)
                        if n > MAX_BYTES:
                            raise TooBig(n)
                        h256.update(chunk)
                        h1.update(chunk)
                        f.write(chunk)
            return part, h256.hexdigest(), h1.hexdigest(), n, final
        except TooBig:
            if os.path.exists(part):
                os.unlink(part)
            raise
        except Exception as e:
            err = repr(e)
            time.sleep(2 * (i + 1))
    if os.path.exists(part):
        os.unlink(part)
    raise RuntimeError("download failed after retries: %s" % err)


class TooBig(Exception):
    pass


def _str(x, cap=200):
    if x is None:
        return None
    return str(x)[:cap]


def _list(x, cap=300):
    if x is None:
        return []
    if isinstance(x, str):
        x = [p.strip() for p in x.split(",")] if "," in x else [x]
    if not isinstance(x, (list, tuple)):
        return [_str(x)]
    out = []
    for v in x[:cap]:
        if isinstance(v, (str, int, float)):
            out.append(str(v)[:200])
    return out


def load_yaml(raw):
    """A descriptor as Spigot reads it, by its byte order mark and YAML 1.1. A document that YAML
    refuses falls back to its top level scalar keys and records the error."""
    text = plugin_yaml.decode(raw)
    try:
        d = plugin_yaml.parse(text)
        if isinstance(d, dict):
            return d, None
        return {}, "yaml is not a mapping"
    except plugin_yaml.DescriptorError as e:
        # minimal fallback: top-level scalar keys only
        d = {}
        for line in text.splitlines():
            m = re.match(r"^([A-Za-z][A-Za-z0-9_-]*):\s*(.*?)\s*$", line)
            if m and m.group(2) and not m.group(2).startswith("#"):
                v = m.group(2).strip().strip("'\"")
                if v.startswith("[") and v.endswith("]"):
                    v = [p.strip().strip("'\"") for p in v[1:-1].split(",") if p.strip()]
                d[m.group(1)] = v
        return d, "yaml error, fallback parser: %s" % str(e).splitlines()[0][:160]


def read_entry(zf, info, cap=2 * 1024 * 1024):
    with zf.open(info) as f:
        return f.read(min(info.file_size, cap))


def inspect_zip(zf, depth=0):
    """Summarise a zip in memory. Entry names are treated as data only."""
    infos = zf.infolist()
    names = {i.filename: i for i in infos}
    res = {
        "entries": len(infos),
        "classes": sum(1 for i in infos if i.filename.endswith(".class")),
        "has_plugin_yml": "plugin.yml" in names,
        "has_paper_plugin_yml": "paper-plugin.yml" in names,
        "has_bungee_yml": "bungee.yml" in names,
        "has_velocity_json": "velocity-plugin.json" in names,
        "has_fabric_json": "fabric.mod.json" in names,
        "has_mods_toml": "META-INF/mods.toml" in names,
    }
    nested = [i for i in infos if i.filename.lower().endswith(".jar") and not i.is_dir()]
    res["nested_jars"] = len(nested)
    res["nested_with_plugin_yml"] = []
    if depth == 0 and not res["has_plugin_yml"] and not res["has_paper_plugin_yml"]:
        for i in nested[:50]:
            if i.file_size > MAX_BYTES:
                continue
            try:
                with zf.open(i) as f:
                    data = f.read(MAX_BYTES + 1)
                with zipfile.ZipFile(io.BytesIO(data)) as inner:
                    if "plugin.yml" in inner.NameToInfo or "paper-plugin.yml" in inner.NameToInfo:
                        res["nested_with_plugin_yml"].append(i.filename[:200])
            except Exception:
                pass
    return res, names


def inspect_jar(path):
    out = {}
    with zipfile.ZipFile(path) as zf:
        summary, names = inspect_zip(zf)
        out.update(summary)
        errs = []
        if summary["has_plugin_yml"]:
            d, e = load_yaml(read_entry(zf, names["plugin.yml"]))
            if e:
                errs.append("plugin.yml: " + e)
            out["plugin_yml"] = {
                "name": _str(d.get("name")),
                "version": _str(d.get("version")),
                "main": _str(d.get("main"), 300),
                "api_version": _str(d.get("api-version")),
                "depend": _list(d.get("depend")),
                "softdepend": _list(d.get("softdepend")),
                "loadbefore": _list(d.get("loadbefore")),
                "libraries": _list(d.get("libraries")),
                "provides": _list(d.get("provides")),
                "authors": _list(d.get("authors")) + _list(d.get("author")),
                "website": _str(d.get("website"), 300),
                "load": _str(d.get("load")),
                "folia_supported": bool(d.get("folia-supported")) if d.get("folia-supported") is not None else None,
                "commands": len(d.get("commands") or {}) if isinstance(d.get("commands"), dict) else 0,
                "permissions": len(d.get("permissions") or {}) if isinstance(d.get("permissions"), dict) else 0,
            }
        if summary["has_paper_plugin_yml"]:
            d, e = load_yaml(read_entry(zf, names["paper-plugin.yml"]))
            if e:
                errs.append("paper-plugin.yml: " + e)
            deps = d.get("dependencies")
            server = {}
            if isinstance(deps, dict):
                for section in ("server", "bootstrap"):
                    sec = deps.get(section)
                    if isinstance(sec, dict):
                        for k, v in list(sec.items())[:300]:
                            req = True
                            if isinstance(v, dict) and v.get("required") is not None:
                                req = bool(v.get("required"))
                            server.setdefault(section, {})[str(k)[:200]] = req
            out["paper_plugin_yml"] = {
                "name": _str(d.get("name")),
                "version": _str(d.get("version")),
                "main": _str(d.get("main"), 300),
                "api_version": _str(d.get("api-version")),
                "bootstrapper": _str(d.get("bootstrapper"), 300),
                "loader": _str(d.get("loader"), 300),
                "authors": _list(d.get("authors")) + _list(d.get("author")),
                "dependencies": server,
            }
        if errs:
            out["parse_errors"] = errs
    return out


# ---------------------------------------------------------------- state

def events_path(source):
    return os.path.join(WORK, source + ".jsonl")


def load_events(source):
    p = events_path(source)
    out = []
    if os.path.exists(p):
        with open(p) as f:
            for line in f:
                line = line.strip()
                if line:
                    out.append(json.loads(line))
    return out


def append_event(source, ev):
    with open(events_path(source), "a") as f:
        f.write(json.dumps(ev, sort_keys=True) + "\n")


class Corpus:
    """Global view of accepted jars, for duplicate detection across sources."""

    def __init__(self):
        self.by_sha256, self.by_sha1, self.by_name = {}, {}, {}  # by_name: lower name -> [events]
        for src in ("live", "spigot", "modrinth", "hangar"):
            for ev in load_events(src):
                if ev["kind"] == "pass":
                    self.add(ev)

    def add(self, ev):
        self.by_sha256[ev["sha256"]] = ev
        self.by_sha1[ev["sha1"]] = ev
        k = name_key(ev)
        if k:
            self.by_name.setdefault(k[0], []).append(ev)

    def same_plugin(self, ev):
        """The kept event that is the same plugin as ev, or None, and why."""
        k = name_key(ev)
        if not k:
            return None, None
        for other in self.by_name.get(k[0], []):
            why = same_plugin(ev, other)
            if why:
                return other, why
        return None, None


def descriptor(ev):
    j = ev.get("jar") or {}
    return j.get("plugin_yml") or j.get("paper_plugin_yml") or {}


def author_tokens(ev):
    out = set()
    for a in descriptor(ev).get("authors") or []:
        for t in re.split(r"[,()\s]+", a.lower()):
            if len(t) >= 3:
                out.add(t)
    return out


GENERIC_HOSTS = {"github", "gitlab", "bitbucket", "codeberg", "sourceforge", "googlecode"}


def package_root(main):
    """First two segments of the main class's package, three when the second
    is a code host (io.github.<user>), so unrelated authors do not match."""
    seg = main.split(".")[:-1]
    n = 3 if len(seg) >= 2 and seg[1].lower() in GENERIC_HOSTS else 2
    return ".".join(seg[:n]) if len(seg) >= n else None


def same_plugin(ev, other):
    """Same plugin.yml name and the same main class, the same package root
    (first two segments of main) or a shared author."""
    a, b = descriptor(ev), descriptor(other)
    if (a.get("name") or "").lower() != (b.get("name") or "").lower():
        return None
    ma, mb = a.get("main") or "", b.get("main") or ""
    if ma == mb:
        return "same plugin name '%s' and main class %s" % (a.get("name"), ma)
    ra, rb = package_root(ma), package_root(mb)
    if ra and ra == rb:
        return "same plugin name '%s' and package root %s (%s vs %s)" % (a.get("name"), ra, ma, mb)
    common = author_tokens(ev) & author_tokens(other)
    if common:
        return "same plugin name '%s' and author %s (%s vs %s)" % (a.get("name"), ", ".join(sorted(common)), ma, mb)
    return None


def name_key(ev):
    """Same plugin = same plugin.yml name (case-insensitive) and same main class."""
    j = ev.get("jar") or {}
    for k in ("plugin_yml", "paper_plugin_yml"):
        y = j.get(k)
        if y and y.get("name"):
            return (y["name"].lower(), y.get("main") or "")
    return None


def plugin_name(ev):
    j = ev.get("jar") or {}
    for k in ("plugin_yml", "paper_plugin_yml"):
        if j.get(k) and j[k].get("name"):
            return j[k]["name"]
    return None


def ref(ev):
    return {"source": ev["source"], "id": ev["id"], "name": ev["name"]}


def accept(corpus, source, meta, url, tag):
    """Download, inspect and record one candidate. Returns 'pass', 'dup' or 'skip'."""
    if dir_bytes() > BUDGET:
        raise SystemExit("disk budget reached")
    try:
        part, h256, h1, n, final = download(url, tag)
    except TooBig as e:
        append_event(source, dict(meta, kind="skip", reason="jar over 60 MB (%d bytes)" % e.args[0], url=url))
        return "skip"
    except Exception as e:
        append_event(source, dict(meta, kind="skip", reason=str(e)[:300], url=url))
        return "skip"
    base = dict(meta, url=url, final_url=final, sha256=h256, sha1=h1, bytes=n)
    if h256 in corpus.by_sha256:
        os.unlink(part)
        append_event(source, dict(base, kind="dup", reason="same sha256", dup_of=ref(corpus.by_sha256[h256])))
        return "dup"
    try:
        info = inspect_jar(part)
    except zipfile.BadZipFile:
        os.unlink(part)
        append_event(source, dict(base, kind="skip", reason="not a zip/jar file"))
        return "skip"
    except Exception as e:
        os.unlink(part)
        append_event(source, dict(base, kind="skip", reason="unreadable jar: %r" % e))
        return "skip"
    if not info["has_plugin_yml"] and not info["has_paper_plugin_yml"]:
        os.unlink(part)
        what = [k[4:] for k in ("has_bungee_yml", "has_velocity_json", "has_fabric_json", "has_mods_toml") if info[k]]
        reason = "no plugin.yml or paper-plugin.yml at the top level"
        if what:
            reason += "; has " + ", ".join(what)
        if info["nested_with_plugin_yml"]:
            reason += "; plugin.yml only inside nested jars: " + ", ".join(info["nested_with_plugin_yml"][:5])
        append_event(source, dict(base, kind="skip", reason=reason, jar=info))
        return "skip"
    ev = dict(base, jar=info)
    other, why = corpus.same_plugin(ev)
    if other:
        os.unlink(part)
        append_event(source, dict(ev, kind="dup", reason=why, dup_of=ref(other)))
        return "dup"
    final_path = os.path.join(JARS, "%s-%s-%s.jar" % (source, safe(meta["id"]), h256[:12]))
    os.replace(part, final_path)
    ev.update(kind="pass", path=final_path)
    append_event(source, ev)
    corpus.add(ev)
    return "pass"


def count(source, kind="pass"):
    return sum(1 for e in load_events(source) if e["kind"] == kind)


def done_ids(source):
    return {str(e["id"]) for e in load_events(source) if e["kind"] in ("pass", "dup", "skip")}


# ---------------------------------------------------------------- live server

def run_live(_target):
    corpus = Corpus()
    seen = done_ids("live")
    items = [
        ("EssentialsX", "2.22.0", "https://github.com/EssentialsX/Essentials/releases/download/2.22.0/EssentialsX-2.22.0.jar"),
        ("EssentialsXSpawn", "2.22.0", "https://github.com/EssentialsX/Essentials/releases/download/2.22.0/EssentialsXSpawn-2.22.0.jar"),
    ]
    for name, ver, url in items:
        if name in seen:
            continue
        meta = {"source": "live", "id": name, "name": name, "version": ver, "downloads": None,
                "page": "https://github.com/EssentialsX/Essentials/releases/tag/2.22.0", "origin": "github release"}
        print(name, accept(corpus, "live", meta, url, "live-" + name), flush=True)
    if "LuckPerms" not in seen:
        vs = get_json("https://api.modrinth.com/v2/project/luckperms/version?loaders=" + quote(json.dumps(["bukkit"])))
        v = [x for x in vs if x["version_number"] == "v5.5.71-bukkit"][0]
        f = [x for x in v["files"] if x.get("primary")][0]
        proj = get_json("https://api.modrinth.com/v2/project/luckperms")
        meta = {"source": "live", "id": "LuckPerms", "name": "LuckPerms", "version": "5.5.71",
                "downloads": proj.get("downloads"), "page": "https://modrinth.com/plugin/luckperms/version/" + v["id"],
                "origin": "modrinth version " + v["id"], "source_sha1": f["hashes"].get("sha1")}
        res = accept(corpus, "live", meta, f["url"], "live-LuckPerms")
        print("LuckPerms", res, flush=True)


# ---------------------------------------------------------------- spigot

def vkey(s):
    parts = re.findall(r"\d+", s)
    return tuple(int(p) for p in parts[:3]) if parts else (0,)


def spigot_120(tested):
    tested = [str(t) for t in (tested or [])]
    if any(t == "1.20" or t.startswith("1.20.") for t in tested):
        return "testedVersions lists 1.20"
    if tested:
        newest = max(tested, key=vkey)
        if vkey(newest) >= (1, 20):
            return "newest tested version %s is at least 1.20" % newest
    return None


def size_bytes(file):
    unit = {"B": 1, "KB": 1024, "MB": 1024 ** 2, "GB": 1024 ** 3}.get((file.get("sizeUnit") or "").upper())
    try:
        return int(float(file.get("size") or 0) * unit) if unit else None
    except Exception:
        return None


def run_spigot(target):
    target = int(target or 250)
    corpus = Corpus()
    seen = done_ids("spigot")
    passes = count("spigot")
    page = 1
    fields = "id,name,tag,file,external,premium,testedVersions,downloads,version,category,updateDate"
    while passes < target:
        lst = get_json("https://api.spiget.org/v2/resources/free?size=200&page=%d&sort=-downloads&fields=%s" % (page, fields))
        if not lst:
            break
        for r in lst:
            if passes >= target:
                break
            rid = str(r["id"])
            if rid in seen:
                continue
            seen.add(rid)
            meta = {"source": "spigot", "id": rid, "name": r.get("name"), "downloads": r.get("downloads"),
                    "page": "https://www.spigotmc.org/resources/%s/" % rid,
                    "category": (r.get("category") or {}).get("id"), "tested_versions": r.get("testedVersions"),
                    "list_rank": None}
            f = r.get("file") or {}
            if r.get("premium"):
                append_event("spigot", dict(meta, kind="skip", reason="premium"))
                continue
            if r.get("external") or f.get("type") != ".jar":
                append_event("spigot", dict(meta, kind="skip", reason="external download" if r.get("external") or f.get("type") == "external" else "file type %s, not .jar" % f.get("type"), file_type=f.get("type")))
                continue
            why = spigot_120(r.get("testedVersions"))
            if not why:
                append_event("spigot", dict(meta, kind="skip", reason="no 1.20 support in testedVersions %s" % (r.get("testedVersions") or [])))
                continue
            sb = size_bytes(f)
            if sb and sb > MAX_BYTES:
                append_event("spigot", dict(meta, kind="skip", reason="jar over 60 MB (listed %s %s)" % (f.get("size"), f.get("sizeUnit"))))
                continue
            vname = None
            try:
                lv = get_json("https://api.spiget.org/v2/resources/%s/versions/latest" % rid)
                vname = (lv or {}).get("name")
            except Exception:
                pass
            meta.update(version=vname, version_id=(r.get("version") or {}).get("id"), support=why)
            res = accept(corpus, "spigot", meta, "https://api.spiget.org/v2/resources/%s/download" % rid, "spigot-" + rid)
            if res == "pass":
                passes += 1
            print("spigot", passes, rid, r.get("name"), res, flush=True)
        page += 1


# ---------------------------------------------------------------- modrinth

def run_modrinth(target):
    target = int(target or 150)
    corpus = Corpus()
    seen = done_ids("modrinth")
    passes = count("modrinth")
    facets = json.dumps([["project_type:plugin"], ["versions:1.20.1"], ["categories:" + l for l in LOADERS]])
    offset = 0
    while passes < target:
        d = get_json("https://api.modrinth.com/v2/search?limit=100&offset=%d&index=downloads&facets=%s" % (offset, quote(facets)))
        hits = (d or {}).get("hits") or []
        if not hits:
            break
        for h in hits:
            if passes >= target:
                break
            pid = h["project_id"]
            if pid in seen:
                continue
            seen.add(pid)
            meta = {"source": "modrinth", "id": pid, "slug": h.get("slug"), "name": h.get("title"),
                    "downloads": h.get("downloads"), "page": "https://modrinth.com/plugin/%s" % h.get("slug")}
            q = "game_versions=%s&loaders=%s" % (quote(json.dumps(["1.20.1"])), quote(json.dumps(LOADERS)))
            try:
                vs = get_json("https://api.modrinth.com/v2/project/%s/version?%s" % (pid, q)) or []
            except Exception as e:
                append_event("modrinth", dict(meta, kind="skip", reason=str(e)[:300]))
                continue
            vs = [v for v in vs if set(v.get("loaders") or []) & set(LOADERS) and "1.20.1" in (v.get("game_versions") or [])]
            if not vs:
                append_event("modrinth", dict(meta, kind="skip", reason="no version for 1.20.1 with loader bukkit/spigot/paper/purpur"))
                continue
            rel = [v for v in vs if v.get("version_type") == "release"]
            v = (rel or vs)[0]
            files = v.get("files") or []
            prim = [f for f in files if f.get("primary")] or files[:1]
            if not prim:
                append_event("modrinth", dict(meta, kind="skip", reason="version has no files"))
                continue
            f = prim[0]
            meta.update(version=v.get("version_number"), version_id=v.get("id"), version_type=v.get("version_type"),
                        loaders=v.get("loaders"), filename_listed=f.get("filename"), source_sha1=(f.get("hashes") or {}).get("sha1"))
            if not (f.get("filename") or "").lower().endswith(".jar"):
                append_event("modrinth", dict(meta, kind="skip", reason="primary file is not a .jar: %s" % f.get("filename")))
                continue
            if (f.get("size") or 0) > MAX_BYTES:
                append_event("modrinth", dict(meta, kind="skip", reason="jar over 60 MB (%s bytes)" % f.get("size")))
                continue
            s1 = (f.get("hashes") or {}).get("sha1")
            if s1 and s1 in corpus.by_sha1:
                append_event("modrinth", dict(meta, kind="dup", reason="same sha1 as listed by Modrinth", url=f["url"], dup_of=ref(corpus.by_sha1[s1])))
                print("modrinth", passes, pid, h.get("title"), "dup", flush=True)
                continue
            res = accept(corpus, "modrinth", meta, f["url"], "modrinth-" + pid)
            if res == "pass":
                passes += 1
            print("modrinth", passes, pid, h.get("title"), res, flush=True)
        offset += 100


# ---------------------------------------------------------------- hangar

def run_hangar(target):
    target = int(target or 100)
    corpus = Corpus()
    seen = done_ids("hangar")
    passes = count("hangar")
    offset = 0
    H = "https://hangar.papermc.io/api/v1"
    while passes < target:
        d = get_json("%s/projects?limit=25&offset=%d&sort=-downloads&platform=PAPER" % (H, offset))
        res_ = (d or {}).get("result") or []
        if not res_:
            break
        for p in res_:
            if passes >= target:
                break
            ns = p.get("namespace") or {}
            slug = ns.get("slug")
            pid = str(p.get("id"))
            if pid in seen:
                continue
            seen.add(pid)
            meta = {"source": "hangar", "id": pid, "slug": slug, "owner": ns.get("owner"), "name": p.get("name"),
                    "downloads": (p.get("stats") or {}).get("downloads"),
                    "page": "https://hangar.papermc.io/%s/%s" % (ns.get("owner"), slug), "category": p.get("category")}
            chosen, pv = None, None
            for pv in ("1.20.1", "1.20"):
                vs = get_json("%s/projects/%s/versions?limit=25&offset=0&platform=PAPER&platformVersion=%s" % (H, quote(slug), pv))
                vs = [v for v in ((vs or {}).get("result") or []) if pv in ((v.get("platformDependencies") or {}).get("PAPER") or [])]
                if vs:
                    stable = [v for v in vs if (v.get("channel") or {}).get("name", "").lower() in ("release", "stable")
                              and "UNSTABLE" not in ((v.get("channel") or {}).get("flags") or [])]
                    chosen = (stable or vs)[0]
                    break
            if not chosen:
                append_event("hangar", dict(meta, kind="skip", reason="no PAPER version for 1.20 or 1.20.1"))
                continue
            dl = ((chosen.get("downloads") or {}).get("PAPER")) or {}
            fi = dl.get("fileInfo") or {}
            meta.update(version=chosen.get("name"), channel=(chosen.get("channel") or {}).get("name"), matched_platform_version=pv,
                        filename_listed=fi.get("name"), source_sha256=fi.get("sha256Hash"))
            if not dl.get("downloadUrl"):
                append_event("hangar", dict(meta, kind="skip", reason="external download: %s" % (dl.get("externalUrl") or "")[:200]))
                continue
            if (fi.get("sizeBytes") or 0) > MAX_BYTES:
                append_event("hangar", dict(meta, kind="skip", reason="jar over 60 MB (%s bytes)" % fi.get("sizeBytes")))
                continue
            if fi.get("sha256Hash") and fi["sha256Hash"] in corpus.by_sha256:
                append_event("hangar", dict(meta, kind="dup", reason="same sha256 as listed by Hangar", url=dl["downloadUrl"], dup_of=ref(corpus.by_sha256[fi["sha256Hash"]])))
                print("hangar", passes, pid, p.get("name"), "dup", flush=True)
                continue
            r = accept(corpus, "hangar", meta, dl["downloadUrl"], "hangar-" + pid)
            if r == "pass":
                passes += 1
            print("hangar", passes, pid, p.get("name"), r, flush=True)
        offset += 25


DIRECT_HOSTS = {"download.geysermc.org", "api.grim.ac"}


def direct_file(url):
    """An external Hangar link that names a file rather than a page."""
    u = urlsplit(url or "")
    if u.scheme != "https":
        return False
    return u.path.lower().endswith(".jar") or (u.hostname or "") in DIRECT_HOSTS


def run_hangar_ext(_t):
    """Download the external Hangar files that are direct links, then keep the
    100 most downloaded Hangar projects that pass."""
    corpus = Corpus()
    evs = load_events("hangar")
    keep, todo = [], []
    for e in evs:
        url = e.get("reason", "")[len("external download: "):]
        if e["kind"] == "skip" and e["reason"].startswith("external download: ") and direct_file(url):
            todo.append((e, url))
        else:
            keep.append(e)
    with open(events_path("hangar") + ".tmp", "w") as f:
        for e in keep:
            f.write(json.dumps(e, sort_keys=True) + "\n")
    os.replace(events_path("hangar") + ".tmp", events_path("hangar"))
    for e, url in todo:
        meta = {k: v for k, v in e.items() if k not in ("kind", "reason")}
        meta["external_url"] = url
        r = accept(corpus, "hangar", meta, url, "hangar-" + e["id"])
        print("hangar-ext", e["id"], e["name"], r, flush=True)
    trim_hangar(100)


def trim_hangar(n):
    evs = load_events("hangar")
    passes = sorted([e for e in evs if e["kind"] == "pass"], key=lambda e: -(e.get("downloads") or 0))
    drop = {e["id"] for e in passes[n:]}
    for e in evs:
        if e["kind"] == "pass" and e["id"] in drop:
            os.unlink(e["path"])
            e.pop("path")
            e.update(kind="skip", reason="beyond the %d most downloaded Hangar projects that pass" % n)
            print("trimmed", e["id"], e["name"], e.get("downloads"))
    with open(events_path("hangar") + ".tmp", "w") as f:
        for e in evs:
            f.write(json.dumps(e, sort_keys=True) + "\n")
    os.replace(events_path("hangar") + ".tmp", events_path("hangar"))


# ---------------------------------------------------------------- re-inspect and re-dedup kept jars

def run_redup(_t):
    """Re-read every kept jar (descriptor fields added later) and apply the
    current duplicate rule to the kept set in source order."""
    srcs = ("live", "spigot", "modrinth", "hangar")
    allev = {s: load_events(s) for s in srcs}
    corpus = Corpus.__new__(Corpus)
    corpus.by_sha256, corpus.by_sha1, corpus.by_name = {}, {}, {}
    changed = 0
    for s in srcs:
        for ev in allev[s]:
            if ev["kind"] != "pass":
                continue
            ev["jar"] = inspect_jar(ev["path"])
            other, why = corpus.same_plugin(ev)
            if other:
                os.unlink(ev["path"])
                print("now duplicate:", s, ev["id"], why, "of", other["source"], other["id"])
                ev.update(kind="dup", reason=why, dup_of=ref(other))
                ev.pop("path")
                changed += 1
            else:
                corpus.add(ev)
        tmp = events_path(s) + ".tmp"
        with open(tmp, "w") as f:
            for ev in allev[s]:
                f.write(json.dumps(ev, sort_keys=True) + "\n")
        os.replace(tmp, events_path(s))
    print("changed", changed)


# ---------------------------------------------------------------- index

def run_index(_t):
    plugins, dups, skips = [], [], []
    for src in ("live", "spigot", "modrinth", "hangar"):
        for ev in load_events(src):
            (plugins if ev["kind"] == "pass" else dups if ev["kind"] == "dup" else skips).append(ev)
    byref = {(p["source"], p["id"]): p for p in plugins}
    for d in dups:
        k = (d["dup_of"]["source"], d["dup_of"]["id"])
        if k in byref:
            byref[k].setdefault("also_in", []).append({x: d.get(x) for x in ("source", "id", "name", "version", "downloads", "url", "sha256", "reason")})
    out_plugins = []
    for p in plugins:
        j = p.get("jar") or {}
        y = j.get("plugin_yml") or {}
        py = j.get("paper_plugin_yml") or {}
        if not os.path.exists(p["path"]):
            raise SystemExit("missing jar " + p["path"])
        rec = {
            "source": p["source"], "id": p["id"], "name": p.get("name"), "version": p.get("version"),
            "downloads": p.get("downloads"), "url": p["url"], "final_url": p.get("final_url"), "page": p.get("page"),
            "sha256": p["sha256"], "sha1": p["sha1"], "bytes": p["bytes"], "path": p["path"],
            "plugin_yml": j.get("has_plugin_yml"), "paper_plugin_yml": j.get("has_paper_plugin_yml"),
            "plugin_name": y.get("name") or py.get("name"), "plugin_version": y.get("version") or py.get("version"),
            "descriptor": "plugin.yml" if y else "paper-plugin.yml",
            "main": y.get("main") if y else py.get("main"),
            "api_version": y.get("api_version") if y else py.get("api_version"),
            "depend": y.get("depend", []), "softdepend": y.get("softdepend", []), "libraries": y.get("libraries", []),
            "loadbefore": y.get("loadbefore", []), "folia_supported": y.get("folia_supported"),
            "authors": (y or py).get("authors", []), "website": y.get("website"),
            "commands": y.get("commands", 0), "permissions": y.get("permissions", 0),
            "paper": ({"main": py.get("main"), "api_version": py.get("api_version"), "bootstrapper": py.get("bootstrapper"),
                       "loader": py.get("loader"), "dependencies": py.get("dependencies")} if py else None),
            "classes": j.get("classes"), "entries": j.get("entries"), "nested_jars": j.get("nested_jars"),
            "also_bungee": j.get("has_bungee_yml"), "also_velocity": j.get("has_velocity_json"),
            "parse_errors": j.get("parse_errors"),
        }
        for k in ("slug", "category", "tested_versions", "support", "version_id", "version_type", "loaders", "channel",
                  "matched_platform_version", "origin", "source_sha1", "source_sha256", "external_url"):
            if p.get(k) is not None:
                rec[k] = p[k]
        if p.get("source_sha1") and p["source_sha1"] != p["sha1"]:
            rec["hash_mismatch"] = "sha1 listed %s, got %s" % (p["source_sha1"], p["sha1"])
        if p.get("source_sha256") and p["source_sha256"] != p["sha256"]:
            rec["hash_mismatch"] = "sha256 listed %s, got %s" % (p["source_sha256"], p["sha256"])
        rec["also_in"] = p.get("also_in", [])
        out_plugins.append(rec)
    counts = {}
    for src in ("live", "spigot", "modrinth", "hangar"):
        c = {"kept": 0, "duplicate": 0, "skipped": 0, "bytes": 0, "skip_reasons": {}}
        for p in out_plugins:
            if p["source"] == src:
                c["kept"] += 1
                c["bytes"] += p["bytes"]
        for d in dups:
            if d["source"] == src:
                c["duplicate"] += 1
        for s in skips:
            if s["source"] == src:
                c["skipped"] += 1
                key = re.sub(r"\(.*?\)|\[.*?\]|: .*$|'[^']*'|\d+(\.\d+)*", "#", s["reason"]).strip()
                c["skip_reasons"][key] = c["skip_reasons"].get(key, 0) + 1
        counts[src] = c
    counts["total"] = {"kept": len(out_plugins), "bytes": sum(p["bytes"] for p in out_plugins),
                       "with_plugin_yml": sum(1 for p in out_plugins if p["plugin_yml"]),
                       "paper_plugin_yml": sum(1 for p in out_plugins if p["paper_plugin_yml"]),
                       "paper_only": sum(1 for p in out_plugins if p["paper_plugin_yml"] and not p["plugin_yml"]),
                       "duplicates": len(dups), "skipped": len(skips)}
    doc = {
        "generated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "purpose": "Corpus of Bukkit/Spigot plugins that run on 1.20.1, for the Golem Bukkit-layer survey (levels 1-3).",
        "jars_dir": JARS,
        "rules": {
            "spigot": "Spiget /resources/free sorted by -downloads; premium, external and non-.jar files skipped; kept when testedVersions lists 1.20 or 1.20.x, or when the newest tested version is 1.20 or later; latest file via /resources/<id>/download",
            "modrinth": "search project_type:plugin, versions:1.20.1, categories bukkit|spigot|paper|purpur, index=downloads; newest release (else newest) version with game version 1.20.1 and one of those loaders; its primary file",
            "hangar": "projects sorted by -downloads with platform PAPER; newest Release-channel (else newest) PAPER version whose platform dependencies list 1.20.1 (else 1.20); its Hangar file, or its external link when that names a file (path ends in .jar, or download.geysermc.org / api.grim.ac), other external links skipped; the 100 most downloaded that pass are kept",
            "live": "EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0 from the GitHub release download URLs, LuckPerms-Bukkit 5.5.71 from Modrinth",
            "dedup": "processed live, spigot, modrinth, hangar in that order; a later jar with the same SHA-256 (or the SHA-1/SHA-256 the source lists) or the same plugin.yml name (case-insensitive) together with the same main class, the same package root (first two segments of the main package, three after io.github and the like) or a shared plugin.yml author is dropped and recorded in also_in of the kept one",
            "kept_requires": "a valid zip with plugin.yml or paper-plugin.yml at the top level, at most 60 MB",
        },
        "counts": counts,
        "plugins": out_plugins,
        "duplicates": [{k: d.get(k) for k in ("source", "id", "name", "version", "downloads", "url", "sha256", "reason", "dup_of")} for d in dups],
        "skipped": [{k: s.get(k) for k in ("source", "id", "name", "version", "downloads", "url", "page", "reason", "tested_versions") if s.get(k) is not None} for s in skips],
    }
    tmp = INDEX + ".tmp"
    with open(tmp, "w") as f:
        json.dump(doc, f, indent=1, sort_keys=False)
    os.replace(tmp, INDEX)
    print(json.dumps(counts, indent=1))


COMMANDS = {"live": run_live, "spigot": run_spigot, "modrinth": run_modrinth, "hangar": run_hangar,
            "hangar_ext": run_hangar_ext, "redup": run_redup, "index": run_index}


def main():
    if len(sys.argv) < 3 or sys.argv[2] not in COMMANDS:
        raise SystemExit("usage: corpus.py <dir> <%s> [target]" % "|".join(COMMANDS))
    set_dir(sys.argv[1])
    os.makedirs(JARS, exist_ok=True)
    os.makedirs(WORK, exist_ok=True)
    COMMANDS[sys.argv[2]](sys.argv[3] if len(sys.argv) > 3 else None)


if __name__ == "__main__":
    main()
