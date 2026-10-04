# tools/isolated_tree_guard.ps1
#
# Shared by tools/sweep_daemon.ps1 and tools/queue_runner.ps1. Dot-source it:
#
#     . (Join-Path $PSScriptRoot "isolated_tree_guard.ps1")
#     $tree = Assert-IsolatedTree -Purpose "queue_runner"
#     $lock = Enter-TreeBuildLock -Tree $tree
#     try { ... build ... } finally { Exit-TreeBuildLock -Lock $lock }
#
# WHY THIS EXISTS
# ---------------
# Both daemons are meant to be long-lived and run unattended. They rebuild and
# re-measure in the BACKGROUND while a human (or another agent) is measuring in
# the foreground. When the tree they operate on is the SAME tree somebody is
# working in, two things go wrong at once:
#
#   * a `make clean` / rebuild from one process deletes the .o files another
#     process is about to link, so the second build links a MIX of old and new
#     objects and the "measurement" is of no build at all;
#   * a data blob gets half-copied, and a byte-spliced ability string table does
#     not raise an error - it just returns the wrong string for every ability
#     from that index on (engine_c_wip/tools/guard_data_blob.sh documents the
#     four times this happened).
#
# That is not hypothetical: contradictory in-tree measurements were the symptom
# that sent this file looking for the cause. The fix is not "be careful", it is
# a gate that cannot be talked past:
#
#   1. ISOLATED_TREE must be set, explicitly, to a directory that is NOT the
#      live tree. Unset, empty, equal to the live tree, nested inside it, or
#      containing it all HARD FAIL. There is no override and no warning mode.
#   2. Builds of that tree take an exclusive lock file, so two daemons cannot
#      build the same snapshot at once even when the snapshot is shared.
#
# The lock is created with FileMode.CreateNew, which is atomic on NTFS, so the
# check and the take cannot interleave.

# NOT setting StrictMode here on purpose: this file is dot-sourced, so it would
# change the semantics of whatever daemon sources it. The guard's own code is
# written to be null-safe instead.

# The live tree these daemons are forbidden to touch: the repository directory
# this script lives in (tools/ -> the repo root).
function Get-LiveRepoRoot {
    return (Split-Path -Parent $PSScriptRoot)
}

function Get-NormalizedPath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) { return "" }
    try { return [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/') }
    catch { return "" }
}

function Assert-IsolatedTree {
    <#
      .SYNOPSIS
        Hard-fail unless $env:ISOLATED_TREE names a directory that is not the
        live tree and is present on disk.
      .OUTPUTS
        The resolved absolute path, with a trailing separator removed.
    #>
    param([string]$Purpose = "this daemon")

    $live = Get-NormalizedPath (Get-LiveRepoRoot)
    $raw  = $env:ISOLATED_TREE

    function Stop([string]$why) {
        $msg = @"
$Purpose REFUSES TO RUN.

  $why

Set ISOLATED_TREE to a SNAPSHOT of the repository and run the daemon against
that, e.g.

  robocopy <live tree> C:\snapshots\enginescan /E /XD .git
  `$env:ISOLATED_TREE = "C:\snapshots\enginescan"
  pwsh tools/$Purpose.ps1

An in-tree run is unsound: the daemon rebuilds and rewrites data blobs while a
foreground build is reading them, and `make clean` deletes the .o files another
build is about to link. This is a hard failure, not a warning - there is no
override.
"@
        Write-Error $msg
        exit 2
    }

    if ([string]::IsNullOrWhiteSpace($raw)) {
        Stop "ISOLATED_TREE is not set."
    }

    $tree = Get-NormalizedPath $raw
    if ($tree -eq "") { Stop "ISOLATED_TREE ($raw) is not a usable path." }

    if ($tree -ieq $live) {
        Stop "ISOLATED_TREE is the LIVE tree ($live)."
    }
    # Nested either way is the same problem: one is inside the other, so a build
    # in one rewrites files the other is reading.
    if ($tree.StartsWith($live + [System.IO.Path]::DirectorySeparatorChar) -or
        $tree.StartsWith($live + "/") -or
        $live.StartsWith($tree + [System.IO.Path]::DirectorySeparatorChar) -or
        $live.StartsWith($tree + "/")) {
        Stop "ISOLATED_TREE ($tree) and the live tree ($live) are nested."
    }

    if (-not (Test-Path -LiteralPath $tree -PathType Container)) {
        Stop "ISOLATED_TREE ($tree) is not a directory on this machine."
    }

    # It has to actually look like the tree the daemon drives, or the failure
    # moves from "unsound measurement" to "cannot find anything to measure".
    if (-not (Test-Path -LiteralPath (Join-Path $tree "engine"))) {
        Stop "ISOLATED_TREE ($tree) has no engine/ directory - that is not a repository snapshot."
    }

    Write-Output $tree
}

function Get-TreeBuildLockPath([string]$Tree) {
    # One lock per tree, so N snapshots build in parallel and 2 processes on the
    # SAME snapshot serialise.
    $dir = Join-Path $Tree ".build-locks"
    if (-not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
    return (Join-Path $dir "tree-build.lock")
}

function Enter-TreeBuildLock {
    <#
      .SYNOPSIS
        Take the exclusive build lock for one tree.
      .DESCRIPTION
        Throws if another live process already holds it. A lock older than
        $StaleMinutes is taken over, because a daemon killed mid-build would
        otherwise wedge the tree forever - and that wedge is exactly the state
        that makes people reach for a stray `rm` in the live tree.
      .OUTPUTS
        A hashtable to hand back to Exit-TreeBuildLock.
    #>
    param(
        [Parameter(Mandatory = $true)][string]$Tree,
        [string]$Label = "build",
        [int]$StaleMinutes = 60,
        [int]$TimeoutSeconds = 0
    )

    $path = Get-TreeBuildLockPath $Tree
    $deadline = if ($TimeoutSeconds -gt 0) { (Get-Date).AddSeconds($TimeoutSeconds) } else { (Get-Date).AddSeconds(15) }
    $holderText = "(unreadable)"
    $holderPid = 0

    while ($true) {
        try {
            # CreateNew is the atomic part: two processes racing here produce one
            # winner and one IOException, never two owners. The share mode is
            # Read, not None: a contender must be able to OPEN the lock to learn
            # who holds it. With None the contender's read fails, it concludes
            # the owner is dead, and it breaks a live lock - which is worse than
            # not locking at all.
            $stream = [System.IO.File]::Open($path, [System.IO.FileMode]::CreateNew,
                                             [System.IO.FileAccess]::Write,
                                             [System.IO.FileShare]::Read)
            $payload = "pid=$PID label=$Label started=$((Get-Date).ToString('s'))"
            $bytes = [System.Text.Encoding]::ASCII.GetBytes($payload)
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush()
            return [pscustomobject]@{ Path = $path; Stream = $stream }
        }
        catch [System.IO.IOException] {
            # Someone holds it. Break a lock whose owner is gone or which is old
            # enough that its owner died holding it.
            $age = [TimeSpan]::FromMinutes(0)
            try {
                $holderText = (Get-Content -LiteralPath $path -Raw -ErrorAction Stop).Trim()
                $age = (Get-Date) - (Get-Item -LiteralPath $path).LastWriteTime
                $pidText = (($holderText -split 'pid=')[1] -split ' ')[0]
                if ($pidText) { [void][int]::TryParse($pidText, [ref]$holderPid) }
            }
            catch {
                $holderText = "(unreadable)"
                $holderPid = 0
            }
            $ownerAlive = $false
            if ($holderPid -gt 0) {
                try { $ownerAlive = $null -ne (Get-Process -Id $holderPid -ErrorAction Stop) }
                catch { $ownerAlive = $false }
            }
            if ($age.TotalMinutes -gt $StaleMinutes -or -not $ownerAlive) {
                Write-Warning ("build lock {0} is stale (holder '{1}', age {2:N0} min) - breaking it." -f `
                    $path, $holderText, $age.TotalMinutes)
                Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
                continue
            }
            if ((Get-Date) -gt $deadline) {
                throw ("build lock {0} is held by a live process (pid {1}, '{2}'). " +
                       "Wait for it, or point ISOLATED_TREE at a different snapshot.") -f `
                       $path, $holderPid, $holderText
            }
            Start-Sleep -Milliseconds 250
        }
    }
}

function Exit-TreeBuildLock($Lock) {
    if ($null -eq $Lock) { return }
    try { $Lock.Stream.Close() } catch { }
    try { Remove-Item -LiteralPath $Lock.Path -Force -ErrorAction SilentlyContinue } catch { }
}