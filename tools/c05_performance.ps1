# No profiling packages. Build once, then synchronously run one real process per
# scenario. Compilation is excluded from child wall/CPU/memory measurements.
[CmdletBinding()]
param(
    [ValidateSet('npc', 'c05')][string[]]$Mode = @('npc', 'c05'),
    [ValidateSet(20, 100, 1000)][int[]]$Actors = @(20, 100, 1000),
    [ValidateRange(1, 100000000)][int]$Ticks = 100,
    [ValidateRange(0, 100000000)][int]$Warmup = 10,
    [ValidateRange(1, 10000)][int]$Iterations = 3,
    [ValidateRange(5, 10000)][int]$MemorySampleIntervalMs = 20,
    [string]$OutputDirectory = (Join-Path $env:LOCALAPPDATA ('Temp\opencode\c05-performance-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))),
    [switch]$SkipBuild,
    [string]$BenchmarkExe,
    # Native snapshots are separate evidence, never inferred from headless runs.
    # Accept JSON objects/arrays, {samples:[...]}, or JSONL; dotted keys supported.
    [string[]]$NativeJson = @(),
    [string]$FrameTimeProperty = 'frame_time_ms',
    [string]$EntityCountProperty = 'entity_count'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$null = New-Item -ItemType Directory -Force -Path $OutputDirectory

function Save-Json($Value, [string]$Path) {
    $json = ConvertTo-Json -InputObject $Value -Depth 40
    [IO.File]::WriteAllText($Path, $json, (New-Object Text.UTF8Encoding($false)))
}

function Read-DottedProperty($Object, [string]$Path) {
    $value = $Object
    foreach ($part in $Path.Split('.')) {
        if ($null -eq $value) { return $null }
        $property = $value.PSObject.Properties[$part]
        if ($null -eq $property) { return $null }
        $value = $property.Value
    }
    return $value
}

function Summarize-Numbers([double[]]$Values) {
    if ($Values.Count -eq 0) { return $null }
    $sorted = @($Values | Sort-Object)
    $measure = $Values | Measure-Object -Average -Maximum -Minimum
    return [ordered]@{
        samples = $Values.Count; mean = $measure.Average; min = $measure.Minimum
        p50 = $sorted[[int][Math]::Ceiling($sorted.Count * 0.50) - 1]
        p95 = $sorted[[int][Math]::Ceiling($sorted.Count * 0.95) - 1]
        p99 = $sorted[[int][Math]::Ceiling($sorted.Count * 0.99) - 1]
        max = $measure.Maximum
    }
}

Push-Location $root
try {
    if (-not $SkipBuild) {
        & cargo build --release --locked --example c05_bench
        if ($LASTEXITCODE -ne 0) { throw 'Release benchmark build failed.' }
    }
    if (-not $BenchmarkExe) {
        $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
        $BenchmarkExe = Join-Path $target 'release\examples\c05_bench.exe'
    }
    $BenchmarkExe = (Resolve-Path -LiteralPath $BenchmarkExe).Path
    $logical = [Environment]::ProcessorCount
    $metadata = [ordered]@{
        schema = 'c05-process-performance-v1'; utc = [DateTime]::UtcNow.ToString('o')
        platform = [Environment]::OSVersion.ToString(); logical_processors = $logical
        processor = $env:PROCESSOR_IDENTIFIER; machine = $env:COMPUTERNAME
        rustc = @(& rustc -Vv); git_head = (& git rev-parse HEAD)
        git_status = @(& git status --short); executable = $BenchmarkExe
        executable_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $BenchmarkExe).Hash.ToLowerInvariant()
        ticks = $Ticks; warmup = $Warmup; iterations = $Iterations
        cpu_scope = 'entire child lifetime including fixture validation, warmup, checkpoint checks, and JSON output; not drive-only'
        memory_scope = 'maximum observed OS lifetime peak working set; lower bound if exit accounting unavailable; not per-tick allocations'
        memory_sample_interval_ms = $MemorySampleIntervalMs
        wall_scope = 'Start-Process plus WaitForExit and redirection; excludes compilation'
    }
    Save-Json $metadata (Join-Path $OutputDirectory 'environment.json')

    $records = @()
    foreach ($selectedMode in $Mode) {
        foreach ($count in $Actors) {
            $name = "$selectedMode-$count"
            $stdout = Join-Path $OutputDirectory "$name.json"
            $stderr = Join-Path $OutputDirectory "$name.stderr.txt"
            $arguments = @('--mode', $selectedMode, '--actors', $count, '--ticks', $Ticks,
                '--warmup', $Warmup, '--iterations', $Iterations)
            $clock = [Diagnostics.Stopwatch]::StartNew()
            $process = Start-Process -FilePath $BenchmarkExe -ArgumentList $arguments -PassThru -NoNewWindow `
                -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            # Keep the native process handle alive so exit accounting remains
            # available where supported. This is a synchronous child-process
            # measurement, not a background job or background agent poll.
            $handle = $process.Handle
            $peak = $null; $memorySamples = 0; $lastMemorySample = $null
            do {
                try {
                    $process.Refresh()
                    $observed = $process.PeakWorkingSet64
                    if ($null -ne $observed -and $observed -gt 0) {
                        if ($null -eq $peak -or $observed -gt $peak) { $peak = $observed }
                        $memorySamples++
                        $lastMemorySample = $clock.Elapsed.TotalSeconds
                    }
                } catch { } # A fast child can exit between refresh and query.
            } while (-not $process.WaitForExit($MemorySampleIntervalMs))
            $process.WaitForExit()
            $clock.Stop()
            $process.Refresh() # Do not label a cached live peak as complete exit data.
            $exitCode = $process.ExitCode
            $cpu = $null; $accountingNotes = @(); $completePeak = $false
            try { $cpu = $process.TotalProcessorTime.TotalSeconds }
            catch { $accountingNotes += 'CPU time unavailable after exit on this host.' }
            try {
                $observed = $process.PeakWorkingSet64
                if ($null -ne $observed -and $observed -gt 0) {
                    if ($null -eq $peak -or $observed -gt $peak) { $peak = $observed }
                    $completePeak = $true
                }
            } catch { }
            if (-not $completePeak) {
                $accountingNotes += 'Exit peak working set unavailable; any live-sampled peak is a lower bound.'
            }
            if ($null -eq $cpu -and $accountingNotes -notcontains 'CPU time unavailable after exit on this host.') {
                $accountingNotes += 'CPU time unavailable after exit on this host.'
            }
            $result = Get-Content -Raw -LiteralPath $stdout | ConvertFrom-Json
            $cpuOneCore = if ($null -ne $cpu) { 100 * $cpu / $clock.Elapsed.TotalSeconds } else { $null }
            $cpuMachine = if ($null -ne $cpuOneCore) { $cpuOneCore / $logical } else { $null }
            $record = [ordered]@{
                mode = $selectedMode; actors = $count; status = $result.status
                exit_code = $exitCode; wall_seconds = $clock.Elapsed.TotalSeconds
                cpu_seconds = $cpu; cpu_percent_one_core = $cpuOneCore; cpu_percent_machine = $cpuMachine
                peak_working_set_bytes = $peak; peak_working_set_complete = $completePeak
                live_memory_observations = $memorySamples; last_live_memory_sample_wall_seconds = $lastMemorySample
                accounting_notes = $accountingNotes
                benchmark_json = $stdout; stderr = $stderr
            }
            $records += $record
            Save-Json $records (Join-Path $OutputDirectory 'process-summary.json')
            $process.Dispose()
            if (($exitCode -ne 0) -and -not (($exitCode -eq 3) -and ($result.status -eq 'unavailable'))) {
                throw "Benchmark $name failed (exit $exitCode). See $stderr"
            }
        }
    }

    $frames = @(); $entities = @(); $nativeSources = @(); $nativeRows = 0
    foreach ($path in $NativeJson) {
        $resolved = (Resolve-Path -LiteralPath $path).Path
        $text = Get-Content -Raw -LiteralPath $resolved
        try { $parsed = $text | ConvertFrom-Json }
        catch { $parsed = @($text -split '\r?\n' | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json }) }
        $sampleProperty = if ($null -ne $parsed) { $parsed.PSObject.Properties['samples'] } else { $null }
        $rows = if ($null -ne $sampleProperty) { @($sampleProperty.Value) } else { @($parsed) }
        foreach ($row in $rows) {
            if ($null -eq $row) { continue }
            $nativeRows++
            $frame = Read-DottedProperty $row $FrameTimeProperty
            $entity = Read-DottedProperty $row $EntityCountProperty
            if ($null -ne $frame) {
                $number = [double]$frame
                if ([double]::IsNaN($number) -or [double]::IsInfinity($number) -or $number -lt 0) { throw "Invalid frame duration in $resolved" }
                $frames += $number
            }
            if ($null -ne $entity) {
                $number = [double]$entity
                if ([double]::IsNaN($number) -or [double]::IsInfinity($number) -or $number -lt 0 -or $number -ne [Math]::Floor($number)) { throw "Invalid entity count in $resolved" }
                $entities += $number
            }
        }
        $nativeSources += [ordered]@{ path = $resolved; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolved).Hash.ToLowerInvariant() }
    }
    Save-Json ([ordered]@{
        status = $(if ($frames.Count -gt 0 -and $entities.Count -gt 0) { 'sampled' } else { 'not_measured_or_missing_fields' })
        sources = $nativeSources; rows = $nativeRows
        frame_time_property = $FrameTimeProperty; entity_count_property = $EntityCountProperty
        frame_time_ms = (Summarize-Numbers $frames); entity_count = (Summarize-Numbers $entities)
        limitation = 'Only supplied observations. Snapshot frame intervals are not GPU timings or a full-frame distribution; no FPS inferred from simulation TPS. Native CPU/memory not measured by the headless child.'
    }) (Join-Path $OutputDirectory 'native-summary.json')
    Write-Output "Performance evidence: $OutputDirectory"
} finally {
    Pop-Location
}
