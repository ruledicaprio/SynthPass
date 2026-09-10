#!/usr/bin/env python3
"""Join SYNTHPASS_OCR_VERBOSE stderr (which pass validated) with the JSON
report (authoritative hit/miss) for default vs band-first."""
import json
import re

OCR = re.compile(r"^\[ocr \d+/\d+\] (.+)$")
GEN = re.compile(r"general pass: ([\d.]+)(ms|s|µs|ns) elapsed")
VAR = re.compile(r"variant \d+: ([\d.]+)(ms|s|µs|ns) elapsed")
VALID = re.compile(r"variant (\d+): valid MRZ found")
GENVALID = re.compile(r"general pass.*valid|Tier-1 hit on general")  # unused; general win emits no line

U = {"ns": 1e-6, "µs": 1e-3, "ms": 1.0, "s": 1000.0}


def parse_log(path):
    d = {}
    cur = None
    for ln in open(path, encoding="utf-8", errors="replace"):
        m = OCR.match(ln.strip())
        if m:
            cur = m.group(1)
            d[cur] = {"pass": 0, "ms": 0.0}
            continue
        if not cur:
            continue
        g = GEN.search(ln)
        if g:
            d[cur]["ms"] += float(g.group(1)) * U[g.group(2)]
        v = VAR.search(ln)
        if v:
            d[cur]["ms"] += float(v.group(1)) * U[v.group(2)]
        vv = VALID.search(ln)
        if vv:
            d[cur]["pass"] = int(vv.group(1))
    return d


def report(path):
    rs = json.load(open(path))["providers"][0]["documents_detail"]
    # positional order matches the log order; key by (index,name) not name
    return rs


dlog = parse_log("artifacts/ocr-order-ab/default-v.err.log")
blog = parse_log("artifacts/ocr-order-ab/band-first-v.err.log")
drep = report("artifacts/ocr-order-ab/default-verbose.json")
brep = report("artifacts/ocr-order-ab/band-first-verbose.json")

names = [r["name"] for r in drep]
assert names == [r["name"] for r in brep]

rows = []
for i, n in enumerate(names):
    dhit = drep[i].get("miss_reason") is None
    bhit = brep[i].get("miss_reason") is None
    dp = dlog.get(n, {}).get("pass", 0)
    bp = blog.get(n, {}).get("pass", 0)
    dms = dlog.get(n, {}).get("ms", 0.0)
    bms = blog.get(n, {}).get("ms", 0.0)
    rows.append((n, dhit, bhit, dp, bp, dms, bms))

# pass distribution over HITS
print("pass distribution (HITS only; pass 0 = validated on the general full-page pass):")
print(f"  {'pass':>6} {'default':>10} {'band-first':>12}")
for p in range(0, 12):
    ca = sum(1 for r in rows if r[1] and r[3] == p)
    cb = sum(1 for r in rows if r[2] and r[4] == p)
    if ca or cb:
        print(f"  {p:>6} {ca:>10} {cb:>12}")
print(f"  {'total':>6} {sum(1 for r in rows if r[1]):>10} {sum(1 for r in rows if r[2]):>12}")

# retries needed = hits that did NOT validate on pass 0
dr = [r for r in rows if r[1] and r[3] > 0]
br = [r for r in rows if r[2] and r[4] > 0]
print(f"\nHITS that needed the retry loop: default {len(dr)}, band-first {len(br)}")
print(f"  sum of validating-pass index: default {sum(r[3] for r in dr)}, "
      f"band-first {sum(r[4] for r in br)}")

# OCR wall-time over docs that entered retries in EITHER arm
retried = [r for r in rows if r[3] > 0 or r[4] > 0]
print(f"\nOCR wall-time over the {len(retried)} docs that hit the retry loop in either arm:")
print(f"  default    {sum(r[5] for r in retried)/1000:>8.1f} s")
print(f"  band-first {sum(r[6] for r in retried)/1000:>8.1f} s")

print("\nper-doc (docs retried in either arm), sorted by default time saved:")
retried.sort(key=lambda r: r[6] - r[5])
for n, dh, bh, dp, bp, dms, bms in retried:
    tag = "" if dh == bh else ("  <== RECOVERED" if bh and not dh else "  <== REGRESSED")
    print(f"  {n[:52]:52} d:p{dp}/{dms/1000:5.1f}s  b:p{bp}/{bms/1000:5.1f}s  D{(bms-dms)/1000:+5.1f}s{tag}")
