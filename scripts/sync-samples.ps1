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
on `main`) into the worktree, commits, and pushes. Uses `robocopy /MIR` so a
local deletion is reflected on `samples-data` too.

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

# robocopy's exit codes 0-7 are all success (e.g. 1 = files copied); only 8+
# is failure. PowerShell 7.3+'s $PSNativeCommandUseErrorActionPreference
# would otherwise turn a normal "files copied" result into a terminating
# error under $ErrorActionPreference = "Stop".
function Invoke-Robocopy {
    param([string[]]$RobocopyArgs)
    robocopy @RobocopyArgs | Out-Null
    if ($LASTEXITCODE -ge 8) {
        throw "robocopy failed (exit $LASTEXITCODE): $($RobocopyArgs -join ' ')"
    }
    $global:LASTEXITCODE = 0
}

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

if (-not $Push) {
    if (-not $branchExists) {
        Write-Host "origin/samples-data does not exist yet -- nothing to pull. Run -Push first."
    } else {
        foreach ($dir in $imageDirs) {
            $src = Join-Path $worktree "samples\$dir"
            $dst = Join-Path $repoRoot "samples\$dir"
            if (Test-Path $src) {
                New-Item -ItemType Directory -Force -Path $dst | Out-Null
                # Additive copy -- never deletes a local file, so re-running is safe.
                Invoke-Robocopy @($src, $dst, "/E", "/XO")
            }
        }
        $src = Join-Path $worktree "samples\ocr_fixtures"
        $dst = Join-Path $repoRoot "samples\ocr_fixtures"
        if (Test-Path $src) {
            New-Item -ItemType Directory -Force -Path $dst | Out-Null
            Invoke-Robocopy @($src, $dst, "/E", "/XO", "/XF", "*.json", "*.md")
        }
        Write-Host "Pulled samples-data into local samples/."
    }
    git worktree remove $worktree --force
    exit 0
}

# --- -Push: mirror local samples/ into the worktree, commit, push. ---------

foreach ($dir in $imageDirs) {
    $src = Join-Path $repoRoot "samples\$dir"
    $dst = Join-Path $worktree "samples\$dir"
    New-Item -ItemType Directory -Force -Path $dst | Out-Null
    if (Test-Path $src) {
        # /MIR so a local deletion is reflected on samples-data too.
        Invoke-Robocopy @($src, $dst, "/MIR")
    }
}

$src = Join-Path $repoRoot "samples\ocr_fixtures"
$dst = Join-Path $worktree "samples\ocr_fixtures"
New-Item -ItemType Directory -Force -Path $dst | Out-Null
if (Test-Path $src) {
    Invoke-Robocopy @($src, $dst, "/MIR", "/XF", "*.json", "*.md")
}

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
