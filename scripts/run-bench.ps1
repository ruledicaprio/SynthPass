<#
.SYNOPSIS
Runs provider-bench (real-specimen tracks) or synthpass-bench (td1/td2/td3/
mrva/mrvb synthetic tracks) scoped to one named "track", appends a flattened
result to the bench-data branch's results/<track>-bench/history.jsonl,
commits, and pushes -- then regenerates that track's trend chart locally.

Real-specimen tracks map to provider-bench's `--format` scoping:
  passport          --format passport   (samples/passports/)
  id_card           --format id_card    (samples/id_cards/)
  driving_license   --format driving_license (samples/driving_licenses/)
  real-specimens    (no --format)       the whole samples/ real corpus

**`passport` above is not the same thing as the synthetic `td3` track
below, despite both meaning "passport-shaped MRZ" in casual speech.**
`passport` scores real photographed specimens under `samples/passports/`
through `provider-bench`; `td3` scores synthpass-gen's own *generated* TD3
corpus through `synthpass-bench` -- same ICAO format, two entirely
different corpora and binaries. This exact collision is what M6 spent its
time untangling (`mrz::Format` vs. `DocumentType::document_code()`, see
knowledge/ROADMAP.md's M6 execution note) -- do not conflate the two tracks
here just because the acronym overlaps.

Synthetic tracks map to synthpass-bench's `--document-type` (the M6 per-format
Tier-1 hit-rate gate, not a real-specimen scope -- see the M6 plan's "Add
td1-bench / td2-bench tracks" step):
  td1               --document-type td1   (synthetic TD1 corpus)
  td2               --document-type td2   (synthetic TD2 corpus)
  td3               --document-type td3   (synthetic TD3 corpus)
  mrva              --document-type mrva  (synthetic MRV-A visa corpus)
  mrvb              --document-type mrvb  (synthetic MRV-B visa corpus)

The five synthetic tracks use a different binary and a different report
shape (synthpass-bench's `hit_rate`, not provider-bench's per-provider
accuracy stats) -- flattened into the *same* history.jsonl row shape as the
real-specimen tracks (`hit_rate` -> `read_ok_rate`, `provider_id` fixed at
`"mrz"`, `field_match_rate`/`mean_cer`/`unsupported_assertion_rate` left
`$null` since synthpass-bench doesn't measure them) so the one `bench-chart`
binary still serves every track without new tooling.

`td3` here is a *second*, independent way to measure the same TD3 format
`.github/workflows/bench-data-collection.yml` already covers nightly into
`dataset.jsonl` -- that workflow's corpus and this track's
`results/td3-bench/history.jsonl` are different files serving different
purposes (a large nightly Tier-1 dataset vs. a small `-Track td1/td2`-shaped
trend point for the per-format comparison chart), not a duplicate schedule.

This is the local, manual counterpart to .github/workflows/bench-data-
collection.yml (which runs synthpass-bench against TD3 only, on a
schedule). Run this by hand whenever you want a fresh data point for a
track; nothing here is scheduled or automatic.

All git writes happen inside an isolated `git worktree` checked out to
bench-data, never in this repo's own working tree -- so a run is safe even
with uncommitted changes sitting on your current branch (this script never
touches them). Mirrors bench-data-collection.yml's exact checkout-or-create
pattern for the bench-data branch.

The chart step shells out to the bench-chart binary and writes
knowledge/img/<track>-bench-trend.svg into *this* working tree. That file is
NOT committed by this script -- main is branch-protected by design, and the
whole point of pushing bench-data instead is keeping unreviewed writes off
it. Review the regenerated SVG and commit it yourself on a normal branch/PR
(knowledge/benchmarks/README.md's live-tracks dashboard is where it's
embedded).

.PARAMETER Track
Which benchmark track to run: passport, id_card, driving_license,
real-specimens, td1, td2, td3, mrva, or mrvb. The last five are synthetic
(synthpass-bench), everything else is a real-specimen provider-bench track
-- see the module doc comment above for why `passport` and `td3` are not
the same thing despite both being "passport-shaped MRZ".

.PARAMETER Limit
Cap on how many specimens to run (provider-bench --limit), applied after
the track's scoping. Real-specimen tracks only; omit to run every specimen
in the track.

.PARAMETER Count
Synthetic tracks only (td1/td2/td3/mrva/mrvb): how many documents to
generate (synthpass-bench --count). Default 100, matching synthpass-bench's
own default.

.PARAMETER Seed
Synthetic tracks only (td1/td2/td3/mrva/mrvb): base seed
(synthpass-bench --seed). Default 0.

.PARAMETER MeasureMemory
Pass through provider-bench --measure-memory (requires building with
--features measure-memory). Real-specimen tracks only.

.PARAMETER SkipChart
Skip regenerating knowledge/img/<track>-bench-trend.svg after the push.

.PARAMETER FromReport
Skip running provider-bench entirely and flatten an existing report JSON
instead (e.g. one produced by a slow manual run you don't want to redo, or a
historical file being folded into this system for the first time). git_sha
defaults to a "backfilled, unknown" placeholder rather than the current
HEAD, since a report produced by an earlier/different run was not
necessarily measured against what's checked out right now -- override with
-GitSha if you do know it.

.PARAMETER GitSha
Override the row's recorded git_sha (only meaningful with -FromReport;
otherwise it's always the current HEAD, which is always true for a fresh
run).

.PARAMETER InvocationNote
Override the row's recorded invocation string (only meaningful with
-FromReport, where the real original invocation may not be known).

.EXAMPLE
./scripts/run-bench.ps1 -Track passport

.EXAMPLE
./scripts/run-bench.ps1 -Track real-specimens -Limit 50

.EXAMPLE
./scripts/run-bench.ps1 -Track td1 -Count 30 -Seed 42

.EXAMPLE
./scripts/run-bench.ps1 -Track real-specimens -FromReport knowledge/benchmarks/qwen-real-50.json -InvocationNote "backfilled from knowledge/benchmarks/qwen-real-50.json"
#>

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("passport", "id_card", "driving_license", "real-specimens", "td1", "td2", "td3", "mrva", "mrvb")]
    [string]$Track,
    [int]$Limit = 0,
    [int]$Count = 100,
    [int]$Seed = 0,
    [switch]$MeasureMemory,
    [switch]$SkipChart,
    [string]$FromReport,
    [string]$GitSha,
    [string]$InvocationNote
)

# Synthetic (synthpass-bench, per-format Tier-1 hit rate) vs real-specimen
# (provider-bench, --format-scoped samples/) track -- decides which binary
# step 1 runs and which report shape step 1.5 flattens.
$isSyntheticTrack = $Track -in @("td1", "td2", "td3", "mrva", "mrvb")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $repoRoot

# --- 1. Run provider-bench (real-specimen tracks) or synthpass-bench -------
# ---    (td1/td2 synthetic tracks), scoped to this track -- or reuse an ----
# ---    existing report via -FromReport.                               -----

if ($FromReport) {
    if (-not (Test-Path $FromReport)) {
        throw "-FromReport path not found: $FromReport"
    }
    $reportPath = (Resolve-Path $FromReport).Path
    Write-Host "Using existing report: $reportPath (skipping the bench run)"
} elseif ($isSyntheticTrack) {
    $reportPath = Join-Path $repoRoot "synthpass-bench-report.json"  # gitignored

    $benchArgs = @("--document-type", $Track, "--profile", "clean", "--count", "$Count", "--seed", "$Seed", "--out", $reportPath)

    Write-Host "Running: cargo run -p synthpass-bench --release --bin synthpass-bench -- $($benchArgs -join ' ')"
    & cargo run -p synthpass-bench --release --bin synthpass-bench -- @benchArgs
    if ($LASTEXITCODE -ne 0) {
        throw "synthpass-bench failed (exit $LASTEXITCODE)"
    }
} else {
    $reportPath = Join-Path $repoRoot "provider-bench-report.json"  # gitignored

    $benchArgs = @("--real-specimens")
    if ($Track -ne "real-specimens") { $benchArgs += @("--format", $Track) }
    $benchArgs += @("--verbose", "--out", $reportPath)
    if ($Limit -gt 0) { $benchArgs += @("--limit", "$Limit") }
    if ($MeasureMemory) { $benchArgs += "--measure-memory" }

    Write-Host "Running: cargo run -p synthpass-bench --release --bin provider-bench -- $($benchArgs -join ' ')"
    & cargo run -p synthpass-bench --release --bin provider-bench -- @benchArgs
    if ($LASTEXITCODE -ne 0) {
        throw "provider-bench failed (exit $LASTEXITCODE)"
    }
}

$report = Get-Content $reportPath -Raw | ConvertFrom-Json
$sha = if ($GitSha) {
    $GitSha
} elseif ($FromReport) {
    "unknown (backfilled from $FromReport, pre-dates git_sha tracking)"
} else {
    (& git rev-parse HEAD).Trim()
}
$invocation = if ($InvocationNote) {
    $InvocationNote
} elseif ($FromReport) {
    "backfilled from $FromReport"
} elseif ($isSyntheticTrack) {
    "synthpass-bench " + ($benchArgs -join " ")
} else {
    "provider-bench " + ($benchArgs -join " ")
}
$runTimestamp = $report.timestamp_unix

if ($isSyntheticTrack) {
    # synthpass-bench has no per-provider breakdown -- it measures exactly
    # one thing, the deterministic Tier-1 (OCR + ICAO 9303 checksum) gate --
    # so this is always a single row, `provider_id` fixed at "mrz" for
    # consistency with the real-specimen tracks' `provider_id` values.
    # `hit_rate` -> `read_ok_rate`: both mean "fraction of documents that
    # produced a usable result," which is what lets `bench-chart` plot every
    # track's `read_ok_rate` on the same trend axis without new tooling.
    # `field_match_rate`/`mean_cer`/`unsupported_assertion_rate` are `$null`
    # (see the module doc comment) rather than fabricated -- synthpass-bench
    # doesn't measure them, and `HistoryRow`'s fields are `#[serde(default)]`
    # precisely so an absent metric stays absent, not a false zero.
    $meanMs = if ($report.results.Count -gt 0) { ($report.results | Measure-Object -Property elapsed_ms -Average).Average } else { $null }
    $rows = @([ordered]@{
        run_timestamp_unix         = $runTimestamp
        git_sha                    = $sha
        invocation                 = $invocation
        documents                  = $report.count
        provider_id                = "mrz"
        read_ok_rate               = $report.hit_rate
        labelled_documents         = $report.count
        field_match_rate           = $null
        mean_cer                   = $null
        unsupported_assertion_rate = $null
        mean_ms                    = $meanMs
    })
} else {
    # One flattened row per provider -- an aggregate per (run, provider), not
    # a per-document row. Matches results/<track>-bench/history.jsonl's
    # schema documented in knowledge/benchmarks/README.md.
    $rows = foreach ($p in $report.providers) {
        $readOkCount = ($p.documents_detail | Where-Object { $_.read_ok }).Count
        $readOkRate = if ($p.documents_detail.Count -gt 0) { $readOkCount / $p.documents_detail.Count } else { $null }
        [ordered]@{
            run_timestamp_unix         = $runTimestamp
            git_sha                    = $sha
            invocation                 = $invocation
            documents                  = $p.documents
            provider_id                = $p.provider_id
            read_ok_rate               = $readOkRate
            labelled_documents         = $p.accuracy.labelled_documents
            field_match_rate           = $p.accuracy.field_match_rate
            mean_cer                   = $p.accuracy.mean_cer
            unsupported_assertion_rate = $p.unsupported_assertion.overall.rate
            mean_ms                    = $p.speed.mean_ms
        }
    }
}

# --- 2. Append to results/<track>/history.jsonl on bench-data, -------------
# ---    entirely inside an isolated worktree.                          -----

$trackDir = "$Track-bench"
$worktree = Join-Path (Split-Path $repoRoot -Parent) "synthpass-bench-data-worktree"
if (Test-Path $worktree) {
    git worktree remove $worktree --force 2>$null
}

# Name the destination ref explicitly. A bare `git fetch origin bench-data`
# updates refs/remotes/origin/bench-data only when `remote.origin.fetch` happens
# to cover it, and CI's `actions/checkout` narrows that refspec to main alone
# (--depth=1, single branch), so the remote-tracking ref is never created there.
git fetch origin "+refs/heads/bench-data:refs/remotes/origin/bench-data" 2>$null

# Branch on the LOCAL ref, not on `git ls-remote`. ls-remote answers "does this
# branch exist on the server"; the line below consumes a remote-tracking ref in
# *this* repo. A shallow single-branch CI checkout makes those two disagree, and
# the old check took the wrong arm: `git worktree add origin/bench-data` then
# died with `fatal: invalid reference` — an hour into the run, after the bench
# had already completed, with the surfaced error being a confusing
# `Push-Location: cannot find path` from the worktree that was never created.
git rev-parse --verify --quiet refs/remotes/origin/bench-data | Out-Null
if ($LASTEXITCODE -eq 0) {
    git worktree add $worktree origin/bench-data | Out-Null
    Push-Location $worktree
    git checkout -B bench-data | Out-Null
} else {
    git worktree add --detach $worktree | Out-Null
    Push-Location $worktree
    git checkout --orphan bench-data | Out-Null
    git rm -rf . 2>$null | Out-Null
}

$resultsDir = Join-Path $worktree "results\$trackDir"
New-Item -ItemType Directory -Force -Path $resultsDir | Out-Null
$historyFile = Join-Path $resultsDir "history.jsonl"

foreach ($row in $rows) {
    ($row | ConvertTo-Json -Compress) | Add-Content -Path $historyFile -Encoding utf8
}

git add "results/$trackDir/history.jsonl"
git diff --cached --quiet
$hasChanges = ($LASTEXITCODE -ne 0)
if ($hasChanges) {
    $totalRows = (Get-Content $historyFile | Measure-Object -Line).Lines
    git commit -m "data: $Track-bench run against main@$sha ($totalRows rows total)" | Out-Null
    git push origin bench-data
    Write-Host "Pushed $($rows.Count) new row(s) to origin/bench-data ($totalRows total for $Track)."
} else {
    Write-Host "No new rows staged -- nothing to commit."
}

Pop-Location

# --- 3. Regenerate the trend chart locally (not committed by this script). -
# Must run before the worktree is removed below -- $historyFile lives inside it.

if (-not $SkipChart) {
    $svgOut = Join-Path $repoRoot "knowledge\img\$Track-bench-trend.svg"
    Write-Host "Regenerating $svgOut ..."
    & cargo run -p synthpass-bench --release --bin bench-chart -- --history $historyFile --out $svgOut
    if ($LASTEXITCODE -eq 0) {
        Write-Host "SVG regenerated. Review it and commit it yourself on a normal branch/PR -- this script never commits to main."
    } else {
        Write-Warning "Chart regeneration failed (exit $LASTEXITCODE) -- the history row was still pushed to bench-data."
    }
}

git worktree remove $worktree --force
