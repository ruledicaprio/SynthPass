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

-Push: mirrors local `samples/{passports,id_cards,driving_licenses,misc}/`
(whole directories -- every file in them is an image) plus the image files
under `samples/ocr_fixtures/` (everything except `*.json`/`*.md`, which stay
on `main`) into the worktree, commits, and pushes. Uses a cross-platform
mirror so a local deletion is reflected on `samples-data` too.

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
    $excludeRegex = if ($ExcludePatterns.Count -gt 0) {
        ($ExcludePatterns -join '|') -replace '\*', '.*'
    } else {
        $null
    }

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
    $excludeRegex = if ($ExcludePatterns.Count -gt 0) {
        ($ExcludePatterns -join '|') -replace '\*', '.*'
    } else {
        $null
    }

    # ---- 1. Remove files/dirs in destination that are not in source ----
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

git fetch origin samples-data 2>$null
git ls-remote --exit-code --heads origin samples-data | Out-Null
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
            Copy-Additive -Source $src -Destination $dst
        }
        $src = Join-Path $worktree "samples\ocr_fixtures"
        $dst = Join-Path $repoRoot "samples\ocr_fixtures"
        # Exclude JSON and Markdown files (they stay on main)
        Copy-Additive -Source $src -Destination $dst -ExcludePatterns @("*.json", "*.md")
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
    Copy-Mirror -Source $src -Destination $dst
}

$src = Join-Path $repoRoot "samples\ocr_fixtures"
$dst = Join-Path $worktree "samples\ocr_fixtures"
# Exclude JSON and Markdown files from the mirror (they stay on main)
Copy-Mirror -Source $src -Destination $dst -ExcludePatterns @("*.json", "*.md")

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