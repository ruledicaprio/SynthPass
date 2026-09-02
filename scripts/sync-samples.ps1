<#
.SYNOPSIS
Syncs specimen images between the local `samples/` corpus and the orphan
`samples-data` branch -- the same isolated-`git worktree` pattern
`scripts/run-bench.ps1` and `.github/workflows/bench-data-collection.yml`
already use for `bench-data`.

.DESCRIPTION
Specimen *images* are not tracked on `main` (see CONTRIBUTING.md) -- only the
small hand-verified ground truth under `samples/ocr_fixtures/*.json`/`*.md`
and `samples/README.md` are. The images themselves live on `samples-data`,
pulled down on demand.

Pull (default): fetches `origin/samples-data` and copies its contents into
local `samples/`, additively -- it never deletes a local file, so it's safe
to run against a working tree that already has extra/staged specimens.

-Push: mirrors the image files under
`samples/{passports,id_cards,driving_licenses,misc,ocr_fixtures}/` into the
worktree, commits, and pushes. `*.json`/`*.md` are excluded from **every** one
of those directories -- hand-verified ground truth is reviewed text that stays
on `main`, wherever in `samples/` it happens to sit next to its image. Uses a
cross-platform mirror, so a local deletion is reflected on `samples-data` too,
and ground truth previously pushed there is cleaned up as an orphan.

All git writes happen inside an isolated worktree, never in this repo's own
working tree -- safe to run with uncommitted changes on your current branch.

.PARAMETER Push
Push the current local corpus to `samples-data` instead of pulling.

.PARAMETER Message
Override the commit message for a -Push run.

.EXAMPLE
./scripts/sync-samples.ps1
Pull the corpus down into local samples/ (e.g. after a fresh clone).

.EXAMPLE
./scripts/sync-samples.ps1 -Push
Push whatever is currently in local samples/ to samples-data.
#>

param(
    [switch]$Push,
    [string]$Message
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $repoRoot

# Whole directories: every file in them is a corpus image.
$imageDirs = @("passports", "id_cards", "driving_licenses", "misc")

# Hand-verified ground truth (`synthpass_core::Extraction` JSON, and the OCR
# Markdown its parity fixtures read) is reviewed text that lives on `main`, not
# image data. It is excluded from *every* mirrored directory, not just
# `ocr_fixtures/` -- ground truth relocated next to its image would otherwise be
# pushed to `samples-data` and silently become the only copy.
$groundTruthPatterns = @("*.json", "*.md")

# ----------------------------------------------------------------------------
# Glob -> anchored regex over a file's *name*.
#
# Escape first, then re-open only `*`, then anchor. Doing it the other way round
# (`$pattern -replace '\*', '.*'`) leaves every other regex metacharacter live:
# "*.json" became ".*.json", where the `.` is a wildcard and the match is
# unanchored, so "Foo_json.jpg" and "xjsonx.png" were excluded too.
# ----------------------------------------------------------------------------
function ConvertTo-NameExcludeRegex {
    param([string[]]$Patterns)

    if ($Patterns.Count -eq 0) { return $null }

    $alternatives = foreach ($p in $Patterns) {
        # [regex]::Escape turns "*.json" into "\*\.json"; reopen the escaped star.
        ([regex]::Escape($p) -replace '\\\*', '.*')
    }
    '^(?:' + ($alternatives -join '|') + ')$'
}

# ----------------------------------------------------------------------------
# Cross-platform replacement for robocopy /E /XO (additive copy, newer wins)
# ----------------------------------------------------------------------------
function Copy-Additive {
    param(
        [string]$Source,
        [string]$Destination,
        [string[]]$ExcludePatterns = @()
    )

    if (-not (Test-Path $Source)) {
        return
    }

    # Ensure destination exists
    if (-not (Test-Path $Destination)) {
        New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    }

    # Build exclusion filter for file names (e.g. "*.json", "*.md")
    $excludeRegex = ConvertTo-NameExcludeRegex -Patterns $ExcludePatterns

    # Recursively get all files from source
    $sourceFiles = Get-ChildItem -Path $Source -File -Recurse

    foreach ($srcFile in $sourceFiles) {
        # Check exclusion
        if ($excludeRegex -and $srcFile.Name -match $excludeRegex) {
            continue
        }

        $relPath = $srcFile.FullName.Substring($Source.Length).TrimStart('\', '/')
        $destFile = Join-Path $Destination $relPath

        # Ensure destination subdirectory exists
        $destDir = Split-Path $destFile -Parent
        if (-not (Test-Path $destDir)) {
            New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        }

        # Copy only if destination doesn't exist or source is newer (/XO semantics)
        if (-not (Test-Path $destFile)) {
            Copy-Item -Path $srcFile.FullName -Destination $destFile -Force
        } else {
            $srcTime = (Get-Item $srcFile.FullName).LastWriteTimeUtc
            $dstTime = (Get-Item $destFile).LastWriteTimeUtc
            if ($srcTime -gt $dstTime) {
                Copy-Item -Path $srcFile.FullName -Destination $destFile -Force
            }
        }
    }
}

# ----------------------------------------------------------------------------
# Cross-platform replacement for robocopy /MIR (mirror, with exclusions)
# ----------------------------------------------------------------------------
function Copy-Mirror {
    param(
        [string]$Source,
        [string]$Destination,
        [string[]]$ExcludePatterns = @()
    )

    if (-not (Test-Path $Source)) {
        # If source doesn't exist, mirror means delete everything in destination
        if (Test-Path $Destination) {
            Remove-Item -Path $Destination -Recurse -Force
        }
        return
    }

    # Ensure destination exists
    if (-not (Test-Path $Destination)) {
        New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    }

    # Build exclusion filter for file names
    $excludeRegex = ConvertTo-NameExcludeRegex -Patterns $ExcludePatterns

    # ---- 1. Remove files/dirs in destination that are not in source ----
    #
    # Deliberately ignores $ExcludePatterns: an excluded file that is already on
    # the branch (e.g. ground truth pushed there before the exclusion was
    # applied to every directory) is an orphan and should be removed. Honouring
    # the exclusion here would strand it on `samples-data` forever.
    $destItems = Get-ChildItem -Path $Destination -Recurse
    foreach ($destItem in $destItems) {
        $relPath = $destItem.FullName.Substring($Destination.Length).TrimStart('\', '/')
        $srcItem = Join-Path $Source $relPath

        # If the source item doesn't exist, the destination item is orphaned
        if (-not (Test-Path $srcItem)) {
            if ($destItem.PSIsContainer) {
                Remove-Item -Path $destItem.FullName -Recurse -Force
            } else {
                Remove-Item -Path $destItem.FullName -Force
            }
        }
    }

    # ---- 2. Copy everything from source (respecting exclusions) ----
    $sourceFiles = Get-ChildItem -Path $Source -File -Recurse

    foreach ($srcFile in $sourceFiles) {
        # Check exclusion
        if ($excludeRegex -and $srcFile.Name -match $excludeRegex) {
            continue
        }

        $relPath = $srcFile.FullName.Substring($Source.Length).TrimStart('\', '/')
        $destFile = Join-Path $Destination $relPath

        $destDir = Split-Path $destFile -Parent
        if (-not (Test-Path $destDir)) {
            New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        }

        Copy-Item -Path $srcFile.FullName -Destination $destFile -Force
    }
}

# --- Worktree setup (unchanged from original) ---

$worktree = Join-Path (Split-Path $repoRoot -Parent) "synthpass-samples-data-worktree"
if (Test-Path $worktree) {
    git worktree remove $worktree --force 2>$null
}

# Explicit refspec + a check on the local ref, for the reason spelled out in
# scripts/run-bench.ps1's matching block: `git ls-remote` asks the server, while
# `git worktree add origin/samples-data` needs the remote-tracking ref to exist
# here. Under CI's shallow single-branch checkout those disagree. This script
# happened to survive that where run-bench.ps1 did not; the difference was luck,
# not design, so both now do the same thing.
git fetch origin "+refs/heads/samples-data:refs/remotes/origin/samples-data" 2>$null
git rev-parse --verify --quiet refs/remotes/origin/samples-data | Out-Null
$branchExists = ($LASTEXITCODE -eq 0)

if ($branchExists) {
    git worktree add $worktree origin/samples-data | Out-Null
    Push-Location $worktree
    git checkout -B samples-data | Out-Null
} else {
    git worktree add --detach $worktree | Out-Null
    Push-Location $worktree
    git checkout --orphan samples-data | Out-Null
    git rm -rf . 2>$null | Out-Null
}
Pop-Location

# --- Pull: additive copy from worktree to local samples/ ---

if (-not $Push) {
    if (-not $branchExists) {
        Write-Host "origin/samples-data does not exist yet -- nothing to pull. Run -Push first."
    } else {
        foreach ($dir in $imageDirs) {
            $src = Join-Path $worktree "samples\$dir"
            $dst = Join-Path $repoRoot "samples\$dir"
            # Cross-platform additive copy (like robocopy /E /XO)
            Copy-Additive -Source $src -Destination $dst -ExcludePatterns $groundTruthPatterns
        }
        $src = Join-Path $worktree "samples\ocr_fixtures"
        $dst = Join-Path $repoRoot "samples\ocr_fixtures"
        Copy-Additive -Source $src -Destination $dst -ExcludePatterns $groundTruthPatterns
        Write-Host "Pulled samples-data into local samples/."
    }
    git worktree remove $worktree --force
    exit 0
}

# --- -Push: mirror local samples/ into the worktree, commit, push. ---

foreach ($dir in $imageDirs) {
    $src = Join-Path $repoRoot "samples\$dir"
    $dst = Join-Path $worktree "samples\$dir"
    # Cross-platform mirror (like robocopy /MIR)
    Copy-Mirror -Source $src -Destination $dst -ExcludePatterns $groundTruthPatterns
}

$src = Join-Path $repoRoot "samples\ocr_fixtures"
$dst = Join-Path $worktree "samples\ocr_fixtures"
Copy-Mirror -Source $src -Destination $dst -ExcludePatterns $groundTruthPatterns

Push-Location $worktree
git add samples
git diff --cached --quiet
$hasChanges = ($LASTEXITCODE -ne 0)
if ($hasChanges) {
    $fileCount = (git ls-files samples | Measure-Object -Line).Lines
    $commitMessage = if ($Message) { $Message } else { "data: sync samples corpus from main@$((git -C $repoRoot rev-parse HEAD).Trim()) ($fileCount files total)" }
    git commit -m $commitMessage | Out-Null
    git push origin samples-data
    Write-Host "Pushed corpus to origin/samples-data ($fileCount files total)."
} else {
    Write-Host "No changes -- nothing to push."
}
Pop-Location

git worktree remove $worktree --force