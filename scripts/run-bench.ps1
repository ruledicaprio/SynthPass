<#
.SYNOPSIS
Runs provider-bench scoped to one named "track", appends the per-provider
result to the bench-data branch's results/<track>/history.jsonl, commits,
and pushes -- then regenerates that track's trend chart locally.

Tracks map to provider-bench's real-specimen scoping:
  passport          --format passport   (samples/passports/)
  id_card           --format id_card    (samples/id_cards/)
  driving_license   --format driving_license (samples/driving_licenses/)
  real-specimens    (no --format)       the whole samples/ real corpus

This is the local, manual counterpart to .github/workflows/bench-data-
collection.yml (which runs the *other* bench binary, synthpass-bench, on a
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
Which benchmark track to run: passport, id_card, driving_license, or
real-specimens.

.PARAMETER Limit
Cap on how many specimens to run (provider-bench --limit), applied after
the track's scoping. Omit to run every specimen in the track.

.PARAMETER MeasureMemory
Pass through provider-bench --measure-memory (requires building with
--features measure-memory).

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
./scripts/run-bench.ps1 -Track real-specimens -FromReport knowledge/benchmarks/qwen-real-50.json -InvocationNote "backfilled from knowledge/benchmarks/qwen-real-50.json"
#>

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("passport", "id_card", "driving_license", "real-specimens")]
    [string]$Track,
    [int]$Limit = 0,
    [switch]$MeasureMemory,
    [switch]$SkipChart,
    [string]$FromReport,
    [string]$GitSha,
    [string]$InvocationNote
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $repoRoot

# --- 1. Run provider-bench, scoped to this track -- or reuse an existing ---
# ---    report via -FromReport.                                        -----

if ($FromReport) {
    if (-not (Test-Path $FromReport)) {
        throw "-FromReport path not found: $FromReport"
    }
    $reportPath = (Resolve-Path $FromReport).Path
    Write-Host "Using existing report: $reportPath (skipping provider-bench run)"
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
} else {
    "provider-bench " + ($benchArgs -join " ")
}
$runTimestamp = $report.timestamp_unix

# One flattened row per provider -- an aggregate per (run, provider), not a
# per-document row. Matches results/<track>/history.jsonl's schema
# documented in knowledge/benchmarks/README.md.
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

# --- 2. Append to results/<track>/history.jsonl on bench-data, -------------
# ---    entirely inside an isolated worktree.                          -----

$trackDir = "$Track-bench"
$worktree = Join-Path (Split-Path $repoRoot -Parent) "synthpass-bench-data-worktree"
if (Test-Path $worktree) {
    git worktree remove $worktree --force 2>$null
}

git fetch origin bench-data 2>$null
git ls-remote --exit-code --heads origin bench-data | Out-Null
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
