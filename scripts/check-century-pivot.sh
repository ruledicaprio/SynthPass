#!/usr/bin/env bash
# Verify crates/mrz's two-digit-year century pivot hasn't drifted too far
# behind the wall clock.
#
#   scripts/check-century-pivot.sh
#
# `crates/mrz` decides whether a two-digit birth year (`YY` in the MRZ's
# `YYMMDD`) belongs to the 1900s or 2000s by comparing it against a hardcoded
# constant, `CURRENT_YY` in `src/dates.rs` — see that file's module doc for
# why: the crate is deterministic and clock-free by design (compiles to
# `wasm32-unknown-unknown`, and a build-time clock read would break
# reproducible builds), so nothing in the crate itself can ever notice the
# pivot going stale. A clock is legitimate in CI, which runs on a schedule and
# is not part of the crate's own build or output — so the staleness check
# lives here instead of in a unit test (a unit test would need a clock, which
# is exactly what the crate refuses to have).
#
# A stale pivot doesn't fail loudly; it misreads quietly. With CURRENT_YY
# stuck at 26, a genuine 2027 birth year (yy=27) rolls back to 1927 once such
# people start presenting documents — a wrong answer with no error, no panic,
# nothing this repository's tests would ever catch.
#
# Failure threshold: more than 2 years behind the current two-digit year.
# Two years is slack for the constant to lag a release cycle without failing
# CI on every push; past that, someone should bump it deliberately (bumping
# is a one-line, patch-level change — see `CURRENT_YY`'s doc comment).
#
# Pure bash. Nothing to install.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

dates_rs="crates/mrz/src/dates.rs"

echo "==> reading CURRENT_YY from $dates_rs"
pivot_line="$(grep -E '^pub const CURRENT_YY: u32 = [0-9]+;' "$dates_rs" || true)"
if [ -z "$pivot_line" ]; then
  echo "::error::could not find 'pub const CURRENT_YY: u32 = N;' in $dates_rs" >&2
  exit 1
fi
pivot_yy="$(printf '%s' "$pivot_line" | sed -E 's/.*=\s*([0-9]+);.*/\1/')"
echo "    CURRENT_YY = $pivot_yy"

echo "==> comparing against the current two-digit year"
current_yy="$((10#$(date -u +%y)))"
echo "    wall-clock year (UTC) = $current_yy"

drift=$((current_yy - pivot_yy))
if [ "$drift" -gt 2 ]; then
  echo "::error::CURRENT_YY ($pivot_yy) in $dates_rs is $drift years behind the current year ($current_yy) — bump it (see the constant's doc comment)" >&2
  exit 1
fi

if [ "$drift" -lt 0 ]; then
  echo "    CURRENT_YY is ahead of the wall clock by $((-drift)) year(s) — fine, not a staleness failure."
fi

echo "CURRENT_YY is within 2 years of the current year."
