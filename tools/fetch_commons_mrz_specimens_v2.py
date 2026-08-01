#!/usr/bin/env python3
"""
fetch_commons_mrz_specimens_v2.py  (FAST version)

Graceful + faster downloader for passport / national ID / visa specimen images
from Wikimedia Commons (for MRZ OCR work).

Changes vs previous version:
- Much tighter search queries → far fewer results
- Slightly lower limits
- Same robust 403/429 handling and clean country-based filenames
"""

import os
import re
import time
import json
from typing import List, Dict, Set, Tuple
from urllib.parse import unquote, quote
import requests

# -------------------- Configuration --------------------
COMMONS_API = "https://commons.wikimedia.org/w/api.php"
DOWNLOAD_DIR = "./downloaded_mrz_documents"
MANIFEST_FILE = os.path.join(DOWNLOAD_DIR, "manifest.json")

START_CATEGORIES = [
    "Category:Passport data pages",
    "Category:Passports by country",
    "Category:National identity cards (EEA)",
    "Category:Identity cards",
    "Category:Biometric passports",
]

# Tighter queries → much fewer (and better) hits
SEARCH_QUERIES = [
    "passport data page specimen",
    "passport specimen biodata",
    "identity card specimen front",
    "national identity card specimen",
    "visa specimen MRZ",
    "machine readable visa specimen",
]

MAX_COUNTRY_SUBCATS = 50          # still good coverage, a bit faster
CM_LIMIT = 40
SEARCH_LIMIT = 30                 # lower = faster

INITIAL_DELAY = 2.0
MAX_DELAY = 20.0
MAX_SINGLE_PAUSE = 45.0
CATEGORY_PAUSE = 1.5
SEARCH_PAUSE = 1.8
DOWNLOAD_PAUSE = 3.0
BATCH_PAUSE = 3.5

HEADERS = {
    "User-Agent": "MRZ-Specimen-Collector/1.4-fast (https://github.com/ruledicaprio/SynthPass; rusmirskopljak@gmail.com)",
    "Accept": "image/webp,image/apng,image/*,*/*;q=0.8",
    "Accept-Language": "en-US,en;q=0.9",
    "Referer": "https://commons.wikimedia.org/",
}

session = requests.Session()
session.headers.update(HEADERS)

CURRENT_DELAY = INITIAL_DELAY
CONSECUTIVE_429 = 0

# Exact country names from your countries.rs
COUNTRY_NAMES = {
    # Africa
    "algeria": "Algeria", "angola": "Angola", "benin": "Benin", "botswana": "Botswana",
    "burkina": "Burkina Faso", "burundi": "Burundi", "cabo verde": "Cabo Verde", "cape verde": "Cabo Verde",
    "cameroon": "Cameroon", "central african": "Central African Republic", "chad": "Chad",
    "comoros": "Comoros", "congo": "Congo", "côte d'ivoire": "Côte d'Ivoire", "ivory coast": "Côte d'Ivoire",
    "djibouti": "Djibouti", "egypt": "Egypt", "equatorial guinea": "Equatorial Guinea",
    "eritrea": "Eritrea", "eswatini": "Eswatini", "ethiopia": "Ethiopia", "gabon": "Gabon",
    "gambia": "Gambia", "ghana": "Ghana", "guinea": "Guinea", "guinea-bissau": "Guinea-Bissau",
    "kenya": "Kenya", "lesotho": "Lesotho", "liberia": "Liberia", "libya": "Libya",
    "madagascar": "Madagascar", "malawi": "Malawi", "mali": "Mali", "mauritania": "Mauritania",
    "mauritius": "Mauritius", "morocco": "Morocco", "mozambique": "Mozambique", "namibia": "Namibia",
    "niger": "Niger", "nigeria": "Nigeria", "rwanda": "Rwanda", "sao tome": "Sao Tome and Principe",
    "senegal": "Senegal", "seychelles": "Seychelles", "sierra leone": "Sierra Leone",
    "somalia": "Somalia", "south africa": "South Africa", "south sudan": "South Sudan",
    "sudan": "Sudan", "tanzania": "Tanzania", "togo": "Togo", "tunisia": "Tunisia",
    "uganda": "Uganda", "zambia": "Zambia", "zimbabwe": "Zimbabwe",

    # Americas
    "antigua": "Antigua and Barbuda", "argentina": "Argentina", "bahamas": "Bahamas",
    "barbados": "Barbados", "belize": "Belize", "bolivia": "Bolivia", "brazil": "Brazil", "brasil": "Brazil",
    "canada": "Canada", "chile": "Chile", "colombia": "Colombia", "costa rica": "Costa Rica",
    "cuba": "Cuba", "dominica": "Dominica", "dominican": "Dominican Republic", "ecuador": "Ecuador",
    "el salvador": "El Salvador", "grenada": "Grenada", "guatemala": "Guatemala", "guyana": "Guyana",
    "haiti": "Haiti", "honduras": "Honduras", "jamaica": "Jamaica", "mexico": "Mexico",
    "nicaragua": "Nicaragua", "panama": "Panama", "paraguay": "Paraguay", "peru": "Peru",
    "saint kitts": "Saint Kitts and Nevis", "saint lucia": "Saint Lucia",
    "saint vincent": "Saint Vincent and the Grenadines", "suriname": "Suriname",
    "trinidad": "Trinidad and Tobago", "united states": "United States of America", "usa": "United States of America",
    "uruguay": "Uruguay", "venezuela": "Venezuela",

    # Asia
    "afghanistan": "Afghanistan", "armenia": "Armenia", "azerbaijan": "Azerbaijan",
    "bahrain": "Bahrain", "bangladesh": "Bangladesh", "bhutan": "Bhutan", "brunei": "Brunei Darussalam",
    "cambodia": "Cambodia", "china": "China", "cyprus": "Cyprus", "georgia": "Georgia",
    "hong kong": "Hong Kong", "india": "India", "indonesia": "Indonesia", "iran": "Iran",
    "iraq": "Iraq", "israel": "Israel", "japan": "Japan", "jordan": "Jordan",
    "kazakhstan": "Kazakhstan", "north korea": "Korea (Democratic People's Republic of)",
    "south korea": "Korea (Republic of)", "korea": "Korea (Republic of)", "kuwait": "Kuwait",
    "kyrgyzstan": "Kyrgyzstan", "laos": "Lao People's Democratic Republic", "lebanon": "Lebanon",
    "macao": "Macao", "macau": "Macao", "malaysia": "Malaysia", "maldives": "Maldives",
    "mongolia": "Mongolia", "myanmar": "Myanmar", "nepal": "Nepal", "oman": "Oman",
    "pakistan": "Pakistan", "palestine": "Palestine", "philippines": "Philippines",
    "qatar": "Qatar", "saudi": "Saudi Arabia", "singapore": "Singapore", "sri lanka": "Sri Lanka",
    "syria": "Syrian Arab Republic", "taiwan": "Taiwan", "tajikistan": "Tajikistan",
    "thailand": "Thailand", "timor": "Timor-Leste", "turkey": "Türkiye", "türkiye": "Türkiye",
    "turkmenistan": "Turkmenistan", "united arab emirates": "United Arab Emirates", "uae": "United Arab Emirates",
    "uzbekistan": "Uzbekistan", "vietnam": "Viet Nam", "viet nam": "Viet Nam", "yemen": "Yemen",

    # Europe
    "albania": "Albania", "andorra": "Andorra", "austria": "Austria", "belarus": "Belarus",
    "belgium": "Belgium", "bosnia": "Bosnia and Herzegovina", "bulgaria": "Bulgaria",
    "croatia": "Croatia", "czechia": "Czechia", "czech": "Czechia", "denmark": "Denmark",
    "estonia": "Estonia", "finland": "Finland", "france": "France", "germany": "Germany",
    "greece": "Greece", "hungary": "Hungary", "iceland": "Iceland", "ireland": "Ireland",
    "italy": "Italy", "kosovo": "Kosovo", "latvia": "Latvia", "liechtenstein": "Liechtenstein",
    "lithuania": "Lithuania", "luxembourg": "Luxembourg", "malta": "Malta", "moldova": "Moldova",
    "monaco": "Monaco", "montenegro": "Montenegro", "netherlands": "Netherlands", "dutch": "Netherlands",
    "north macedonia": "North Macedonia", "macedonia": "North Macedonia", "norway": "Norway",
    "poland": "Poland", "portugal": "Portugal", "romania": "Romania", "russia": "Russian Federation",
    "san marino": "San Marino", "serbia": "Serbia", "slovakia": "Slovakia", "slovenia": "Slovenia",
    "spain": "Spain", "sweden": "Sweden", "switzerland": "Switzerland", "ukraine": "Ukraine",
    "united kingdom": "United Kingdom", "uk": "United Kingdom", "british": "United Kingdom",
    "vatican": "Holy See (Vatican City State)",

    # Oceania
    "australia": "Australia", "fiji": "Fiji", "kiribati": "Kiribati", "marshall": "Marshall Islands",
    "micronesia": "Micronesia (Federated States of)", "nauru": "Nauru", "new zealand": "New Zealand",
    "palau": "Palau", "papua": "Papua New Guinea", "samoa": "Samoa", "solomon": "Solomon Islands",
    "tonga": "Tonga", "tuvalu": "Tuvalu", "vanuatu": "Vanuatu",
}

# -------------------- Helpers --------------------
def make_nice_filename(commons_title: str, url: str = "") -> str:
    name = commons_title.replace("File:", "", 1)
    name = unquote(name)
    lower = name.lower()

    country = "Unknown"
    for key, official in COUNTRY_NAMES.items():
        if key in lower:
            country = official
            break

    if any(x in lower for x in ["passport", "passeport", "passaporte", "reisepass", "pasaporte"]):
        doc_type = "Passport"
    elif any(x in lower for x in [
        "identity card", "id card", "id_card", "personalausweis",
        "carte d'identité", "carta d'identità", "dni", "cédula",
        "national identity", "identity_card"
    ]):
        doc_type = "ID"
    elif "visa" in lower:
        doc_type = "Visa"
    else:
        doc_type = "Document"

    if any(x in lower for x in ["front", "recto", "obverse", "avers", "vorderseite"]):
        side = "front"
    elif any(x in lower for x in ["back", "verso", "reverse", "reverso", "rückseite"]):
        side = "back"
    elif any(x in lower for x in ["data page", "biodata", "identity page", "personal data"]):
        side = "data_page"
    else:
        side = "specimen"

    clean = re.sub(r'(?i)file:', '', name)
    clean = re.sub(
        r'(?i)(passport|identity.?card|id.?card|visa|specimen|sample|front|back|recto|verso|data.?page|national)',
        '', clean
    )
    clean = re.sub(r'[^\w\s-]', '', clean)
    clean = re.sub(r'\s+', '_', clean).strip('_')
    clean = re.sub(r'_+', '_', clean)
    if len(clean) > 35:
        clean = clean[:35].rstrip('_')

    ext = os.path.splitext(url.split("?")[0])[1].lower() if url else ""
    if not ext or len(ext) > 5:
        ext = ".jpg"

    parts = [country.replace(" ", "_"), doc_type, "Specimen", side]
    if clean:
        parts.append(clean)

    return "_".join(parts) + ext


def fetch_with_retry(url: str, params: dict = None, max_retries: int = 6, stream: bool = False):
    global CURRENT_DELAY, CONSECUTIVE_429

    for attempt in range(max_retries):
        try:
            resp = session.get(url, params=params, timeout=30, stream=stream)
            if resp.status_code == 200:
                CURRENT_DELAY = max(INITIAL_DELAY, CURRENT_DELAY * 0.85)
                CONSECUTIVE_429 = 0
                return resp

            if resp.status_code == 429:
                CONSECUTIVE_429 += 1
                CURRENT_DELAY = min(MAX_DELAY, CURRENT_DELAY * 1.8)
                extra = min(40.0, 7.0 * CONSECUTIVE_429)
                pause = min(MAX_SINGLE_PAUSE, CURRENT_DELAY * (1.6 ** attempt) + extra)
                print(f"  [!] 429 – sleeping {pause:.1f}s (try {attempt+1}/{max_retries})")
                time.sleep(pause)
                continue

            resp.raise_for_status()

        except requests.RequestException as e:
            if attempt == max_retries - 1:
                print(f"  [!] Request failed: {e}")
                return None
            pause = min(MAX_SINGLE_PAUSE, CURRENT_DELAY * (1.7 ** attempt))
            print(f"  [!] Network error, sleeping {pause:.1f}s …")
            time.sleep(pause)

    return None


def download_with_grace(url: str, filepath: str, max_retries: int = 5) -> bool:
    if os.path.exists(filepath):
        print(f"  [=] Exists: {os.path.basename(filepath)}")
        return True

    for attempt in range(max_retries):
        try:
            resp = session.get(url, stream=True, timeout=30, allow_redirects=True)

            if resp.status_code == 200:
                with open(filepath, "wb") as f:
                    for chunk in resp.iter_content(chunk_size=16384):
                        if chunk:
                            f.write(chunk)
                print(f"  [+] Saved: {os.path.basename(filepath)}")
                return True

            if resp.status_code == 403:
                print(f"  [!] 403 Forbidden (attempt {attempt+1}/{max_retries})")
                if attempt == 0 and "upload.wikimedia.org" in url:
                    filename = url.split("/")[-1].split("?")[0]
                    alt_url = f"https://commons.wikimedia.org/wiki/Special:FilePath/{quote(filename)}"
                    print(f"      Trying fallback Special:FilePath …")
                    url = alt_url
                    time.sleep(2)
                    continue
                time.sleep(8 + attempt * 5)

            elif resp.status_code == 429:
                pause = min(90, 10 * (2 ** attempt))
                print(f"  [!] 429 – sleeping {pause}s")
                time.sleep(pause)

            else:
                print(f"  [!] HTTP {resp.status_code}")
                time.sleep(5)

        except requests.RequestException as e:
            print(f"  [!] Network error: {e}")
            time.sleep(5 + attempt * 3)

    print(f"  [-] Failed after retries: {url}")
    return False


# -------------------- Commons API --------------------
def get_category_members(category: str, cmtype: str = "file|subcat") -> Tuple[List[str], List[str]]:
    files, subcats = [], []
    continue_param = None

    while True:
        params = {
            "action": "query",
            "list": "categorymembers",
            "cmtitle": category,
            "cmlimit": CM_LIMIT,
            "cmtype": cmtype,
            "format": "json",
        }
        if continue_param:
            params["cmcontinue"] = continue_param

        resp = fetch_with_retry(COMMONS_API, params=params)
        if not resp:
            break

        data = resp.json()
        for m in data.get("query", {}).get("categorymembers", []):
            if m["ns"] == 6:
                files.append(m["title"])
            elif m["ns"] == 14:
                subcats.append(m["title"])

        if "continue" in data:
            continue_param = data["continue"]["cmcontinue"]
            time.sleep(0.6)
        else:
            break

    return files, subcats


def search_files(query: str) -> List[str]:
    titles = []
    continue_param = None

    while True:
        params = {
            "action": "query",
            "list": "search",
            "srsearch": query,
            "srnamespace": 6,
            "srlimit": SEARCH_LIMIT,
            "format": "json",
        }
        if continue_param is not None:
            params["sroffset"] = continue_param

        resp = fetch_with_retry(COMMONS_API, params=params)
        if not resp:
            break

        data = resp.json()
        for r in data.get("query", {}).get("search", []):
            titles.append(r["title"])

        if "continue" in data:
            continue_param = data["continue"]["sroffset"]
            time.sleep(0.7)
        else:
            break

    return titles


def get_image_info(titles: List[str]) -> List[Dict]:
    results = []
    for i in range(0, len(titles), 35):
        batch = titles[i:i+35]
        params = {
            "action": "query",
            "titles": "|".join(batch),
            "prop": "imageinfo",
            "iiprop": "url|size|mime",
            "format": "json",
        }
        resp = fetch_with_retry(COMMONS_API, params=params)
        if not resp:
            continue

        for page in resp.json().get("query", {}).get("pages", {}).values():
            if "imageinfo" not in page:
                continue
            info = page["imageinfo"][0]
            mime = info.get("mime", "")
            if not mime.startswith("image/") or "svg" in mime.lower():
                continue
            url = info.get("url")
            if not url:
                continue
            results.append({
                "title": page.get("title", "unknown"),
                "url": url,
                "mime": mime,
                "size": info.get("size", 0),
            })
        time.sleep(BATCH_PAUSE)
    return results


# -------------------- Main --------------------
def main():
    os.makedirs(DOWNLOAD_DIR, exist_ok=True)

    all_file_titles: Set[str] = set()
    seen_cats: Set[str] = set()

    print("=== Collecting file titles from categories (FAST mode) ===\n")

    for cat in START_CATEGORIES:
        if cat in seen_cats:
            continue
        seen_cats.add(cat)
        print(f"Category: {cat}")
        files, subcats = get_category_members(cat)
        all_file_titles.update(files)
        print(f"  → {len(files)} files, {len(subcats)} subcats")
        time.sleep(CATEGORY_PAUSE)

        if "Passports by country" in cat:
            print(f"\n  Expanding up to {MAX_COUNTRY_SUBCATS} country subcategories…")
            for i, sub in enumerate(subcats[:MAX_COUNTRY_SUBCATS], 1):
                if sub in seen_cats:
                    continue
                seen_cats.add(sub)
                print(f"  [{i}/{min(len(subcats), MAX_COUNTRY_SUBCATS)}] {sub}")
                f2, _ = get_category_members(sub, cmtype="file")
                all_file_titles.update(f2)
                print(f"      → {len(f2)} files")
                time.sleep(CATEGORY_PAUSE)

    print("\n=== Collecting file titles from search queries (tight) ===\n")
    for q in SEARCH_QUERIES:
        print(f"Search: {q}")
        found = search_files(q)
        all_file_titles.update(found)
        print(f"  → {len(found)} hits")
        time.sleep(SEARCH_PAUSE)

    print(f"\nTotal unique File: titles: {len(all_file_titles)}")
    file_list = sorted(all_file_titles)

    print("Fetching image metadata…")
    image_infos = get_image_info(file_list)
    print(f"Usable images: {len(image_infos)}\n")

    # Load previous manifest
    manifest = {}
    if os.path.exists(MANIFEST_FILE):
        try:
            with open(MANIFEST_FILE, "r", encoding="utf-8") as f:
                manifest = json.load(f)
        except Exception:
            pass

    print(f"=== Downloading to {DOWNLOAD_DIR} ===\n")
    downloaded = skipped = 0

    for idx, info in enumerate(image_infos, 1):
        title = info["title"]
        url = info["url"]
        safe_name = make_nice_filename(title, url)
        filepath = os.path.join(DOWNLOAD_DIR, safe_name)

        print(f"[{idx}/{len(image_infos)}] {title}")
        print(f"         → {safe_name}")

        success = download_with_grace(url, filepath)

        if success:
            if safe_name not in manifest:
                downloaded += 1
            else:
                skipped += 1
            manifest[safe_name] = {
                "commons_title": title,
                "url": url,
                "size": info.get("size"),
                "mime": info.get("mime"),
            }
        else:
            print("         [-] download failed")

        time.sleep(DOWNLOAD_PAUSE)

        if downloaded % 15 == 0 and downloaded > 0:
            with open(MANIFEST_FILE, "w", encoding="utf-8") as f:
                json.dump(manifest, f, indent=2, ensure_ascii=False)

    with open(MANIFEST_FILE, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    print("\n=== Finished ===")
    print(f"Downloaded : {downloaded}")
    print(f"Skipped    : {skipped}")
    print(f"Manifest   : {MANIFEST_FILE}")
    print(f"Folder     : {DOWNLOAD_DIR}")


if __name__ == "__main__":
    main()