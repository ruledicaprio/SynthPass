<#
.SYNOPSIS
Sifts a Commons fetch staging directory (tools/downloaded_mrz_documents/ or
tools/downloaded_passports_and_ids/, both gitignored) into samples/, applying
automatically the same provenance rule CONTRIBUTING.md's "Adding a corpus
specimen" checklist used to ask a human to apply by eye.

.DESCRIPTION
This replaces the *human PR-review* step, not the check itself:
CLAUDE.md's "never bypass validation" still holds. A candidate is
auto-accepted only if `check_sample` reports a genuine signal per
CONTRIBUTING.md's existing rule --

  1. the OCR text contains "specimen" (a printed SPECIMEN watermark), OR
  2. the MRZ that was found uses an established synthetic-placeholder
     document number already used in this corpus (E00000000, 000000000,
     007007007)

-- and the OCR text doesn't match a known novelty/fake-ID-vendor watermark
(CONTRIBUTING.md names mrpassports.com-style sites as a specific reject).
Everything else is left untouched in the staging directory, logged as
rejected, for a human to look at by hand if they want to.

This does NOT touch `crates/synthpass-ocr/examples/mrz_corpus.rs`'s `CORPUS`/
`NEGATIVE` ground truth or `samples/ocr_fixtures/`'s hand-verified JSON --
reading off the real document number for those requires a human, and stays a
separate, smaller, opt-in step (see CONTRIBUTING.md).

Accepted files are renamed to `Country_DocType_Specimen[_Variant].ext`
(DocType classified from the fetch script's own filename convention --
`Passport`/`ID`/`Driving`/else `misc`), placed in the matching samples/
subdirectory (checked for basename collisions against the rest of
samples/), and pushed to samples-data via scripts/sync-samples.ps1 -Push.

.PARAMETER StagingDir
Directory to sift. Defaults to tools/downloaded_mrz_documents (the current
fetch script's staging dir) if present, else tools/downloaded_passports_and_ids.

.PARAMETER DryRun
Report accept/reject decisions without moving any files or pushing.

.PARAMETER SkipPush
Move accepted files into samples/ but don't push to samples-data (e.g. to
batch several ingest runs before one push).

.EXAMPLE
./scripts/ingest-fetched-samples.ps1 -DryRun

.EXAMPLE
./scripts/ingest-fetched-samples.ps1
#>

param(
    [string]$StagingDir,
    [switch]$DryRun,
    [switch]$SkipPush
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $repoRoot

if (-not $StagingDir) {
    if (Test-Path "tools/downloaded_mrz_documents") {
        $StagingDir = "tools/downloaded_mrz_documents"
    } elseif (Test-Path "tools/downloaded_passports_and_ids") {
        $StagingDir = "tools/downloaded_passports_and_ids"
    } else {
        Write-Host "No staging directory found (tools/downloaded_mrz_documents or tools/downloaded_passports_and_ids). Nothing to ingest."
        exit 0
    }
}
if (-not (Test-Path $StagingDir)) {
    throw "-StagingDir not found: $StagingDir"
}

# Established synthetic-placeholder MRZ document numbers already used in this
# corpus -- see CONTRIBUTING.md "Adding a corpus specimen", point 1.
$placeholderDocNumbers = @("E00000000", "000000000", "007007007")

# CONTRIBUTING.md names novelty/fake-ID-document vendor sites as a specific
# reject even when a "specimen"-shaped watermark is present. Best-effort,
# not exhaustive -- extend as new vendors are seen.
$vendorBlocklist = @("mrpassports", "novelty", "fakeid", "fake-id")

$extensions = @(".jpg", ".jpeg", ".png", ".webp", ".gif")

$imageFiles = Get-ChildItem -Path $StagingDir -File | Where-Object {
    $extensions -contains $_.Extension.ToLowerInvariant()
}
if ($imageFiles.Count -eq 0) {
    Write-Host "No image files in $StagingDir. Nothing to ingest."
    exit 0
}

# Existing basenames across the whole corpus -- lookups are basename-based
# (CONTRIBUTING.md), so a new file must not collide with any of them.
$existingBasenames = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($f in Get-ChildItem -Path "samples" -Recurse -File -Include *.jpg, *.jpeg, *.png, *.webp, *.gif) {
    $existingBasenames.Add($f.Name.ToLowerInvariant()) | Out-Null
}

function Get-DocTypeDir([string]$FileName) {
    # Mirrors tools/fetch_commons_mrz_specimens_v2.py's own doc_type tagging,
    # which already embeds Passport/ID/Driving/Document into the filename.
    if ($FileName -match "(?i)passport") { return "passports" }
    if ($FileName -match "(?i)driving|licen[cs]e") { return "driving_licenses" }
    if ($FileName -match "(?i)\bid\b|identity") { return "id_cards" }
    return "misc"
}

Write-Host "Building provider-bench / check_sample in release mode (first run only)..."
cargo build -p synthpass-ocr --release --example check_sample | Out-Null
if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
$checkSampleExe = "target/release/examples/check_sample.exe"

$accepted = 0
$rejected = 0

foreach ($file in $imageFiles) {
    $output = & $checkSampleExe $file.FullName 2>&1 | Out-String

    $hasSpecimenText = $output -match 'WATERMARK\s+OCR text contains "specimen"'
    $hasPlaceholderDoc = $false
    foreach ($doc in $placeholderDocNumbers) {
        if ($output -match "MRZ HIT.*doc_number=$doc\b") { $hasPlaceholderDoc = $true }
    }
    $hitsVendorBlocklist = $false
    foreach ($vendor in $vendorBlocklist) {
        if ($output -match "(?i)$vendor") { $hitsVendorBlocklist = $true }
    }

    $accept = ($hasSpecimenText -or $hasPlaceholderDoc) -and (-not $hitsVendorBlocklist)

    if (-not $accept) {
        $rejected++
        Write-Host "REJECT  $($file.Name)  (specimen_text=$hasSpecimenText placeholder_doc=$hasPlaceholderDoc vendor_blocklist=$hitsVendorBlocklist)" -ForegroundColor DarkYellow
        continue
    }

    if ($existingBasenames.Contains($file.Name.ToLowerInvariant())) {
        $rejected++
        Write-Host "REJECT  $($file.Name)  (basename collision with an existing samples/ file)" -ForegroundColor DarkYellow
        continue
    }

    $docTypeDir = Get-DocTypeDir $file.Name
    $destDir = Join-Path $repoRoot "samples\$docTypeDir"
    $destPath = Join-Path $destDir $file.Name

    Write-Host "ACCEPT  $($file.Name) -> samples/$docTypeDir/" -ForegroundColor Green
    $accepted++
    $existingBasenames.Add($file.Name.ToLowerInvariant()) | Out-Null

    if (-not $DryRun) {
        New-Item -ItemType Directory -Force -Path $destDir | Out-Null
        Move-Item -Path $file.FullName -Destination $destPath -Force
    }
}

Write-Host "`n$accepted accepted, $rejected rejected (of $($imageFiles.Count) candidates)."

if ($DryRun) {
    Write-Host "Dry run -- no files moved, nothing pushed."
    exit 0
}

if ($accepted -eq 0) {
    Write-Host "Nothing accepted -- skipping push."
    exit 0
}

if ($SkipPush) {
    Write-Host "-SkipPush set -- accepted files are in samples/ but not pushed to samples-data yet."
    exit 0
}

& "$PSScriptRoot/sync-samples.ps1" -Push -Message "data: ingest $accepted specimen(s) from $StagingDir"
