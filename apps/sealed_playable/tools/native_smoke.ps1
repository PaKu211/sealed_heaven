param([string]$EvidenceDirectory = (Join-Path $env:TEMP ('sealed-c04-smoke-' + (Get-Date -Format yyyyMMdd-HHmmss))), [switch]$KeyboardOnly)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'native_window.ps1')
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$exe = Join-Path $root 'apps/sealed_playable/target/debug/sealed_playable.exe'
New-Item -ItemType Directory -Force $EvidenceDirectory | Out-Null
$oldLocal=$env:LOCALAPPDATA
$oldEvidence=$env:SEALED_HEAVEN_EVIDENCE_DIR
$env:LOCALAPPDATA=$EvidenceDirectory
$env:SEALED_HEAVEN_EVIDENCE_DIR=$EvidenceDirectory
try {
    $process=Start-Process -FilePath $exe -PassThru -WorkingDirectory $root -RedirectStandardOutput (Join-Path $EvidenceDirectory 'stdout.log') -RedirectStandardError (Join-Path $EvidenceDirectory 'stderr.log')
} finally { $env:LOCALAPPDATA=$oldLocal; $env:SEALED_HEAVEN_EVIDENCE_DIR=$oldEvidence }
$process.Id | Set-Content (Join-Path $EvidenceDirectory 'pid.txt')
$script:sequence=0
$script:last=$null
$notes=New-Object 'System.Collections.Generic.List[string]'
function Assert-True($condition,[string]$message) { if (-not $condition) { throw $message } }
function Snapshot([string]$name) {
    $process.Refresh()
    Prepare-AppWindow $process.MainWindowHandle
    $script:sequence++
    $base=Join-Path $EvidenceDirectory ('native-{0:000}' -f $script:sequence)
    Post-AppKey $process.MainWindowHandle 0x7B 0x58
    $clock=[Diagnostics.Stopwatch]::StartNew()
    $png=$null
    while ($null -eq $png) {
        if ($clock.Elapsed.TotalSeconds -gt 15) { throw "Framebuffer capture timed out: $name" }
        if ((Test-Path "$base.json") -and (Test-Path "$base.png")) {
            try {
                $candidate=[IO.File]::ReadAllBytes("$base.png")
                if ($candidate.Length -ge 24 -and [Text.Encoding]::ASCII.GetString($candidate,$candidate.Length-8,4) -eq 'IEND') {
                    $png=$candidate
                    break
                }
            } catch [IO.IOException] { } # async renderer may still own the file
        }
        Start-Sleep -Milliseconds 100
    }
    $script:last=Get-Content -Raw -Encoding UTF8 "$base.json" | ConvertFrom-Json
    $width=($png[16]*16777216)+($png[17]*65536)+($png[18]*256)+$png[19]
    $height=($png[20]*16777216)+($png[21]*65536)+($png[22]*256)+$png[23]
    Assert-True ($width -ge 1200 -and $height -ge 700) 'Native framebuffer is minimized or too small for interactive verification'
    Assert-True ($script:last.actors -eq 20) 'Expected twenty authoritative NPCs'
    Assert-True ($script:last.diagnostic -eq '') "Application error: $($script:last.diagnostic)"
    $notes.Add("$name => native-$('{0:000}' -f $script:sequence), tick=$($script:last.tick), selected=$($script:last.selected), paused=$($script:last.paused), hash=$($script:last.hash)")
}
function Button([string]$control) {
    $process.Refresh()
    Prepare-AppWindow $process.MainWindowHandle
    if ($KeyboardOnly) {
        $key = switch ($control) {
            'Toggle' { @(0x20,0x39) }; 'Step' { @(0x4E,0x31) }
            'Save' { @(0x74,0x3F) }; 'Load' { @(0x78,0x43) }; 'Reset' { @(0x52,0x13) }
            'English' { if($last.locale -eq 'English') { return }; @(0x4C,0x26) }
            'Indonesian' { if($last.locale -eq 'Indonesian') { return }; @(0x4C,0x26) }
            default { throw "No keyboard fallback: $control" }
        }
        Post-AppKey $process.MainWindowHandle $key[0] $key[1]
        Start-Sleep -Milliseconds 250
        return
    }
    $button=$script:last.buttons | Where-Object control -eq $control | Select-Object -First 1
    Assert-True ($null -ne $button) "Missing UI button: $control"
    Post-AppClick $process.MainWindowHandle ([int]$button.x) ([int]$button.y)
    Start-Sleep -Milliseconds 250
}
try {
    $startup=[Diagnostics.Stopwatch]::StartNew()
    do {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        if ($process.HasExited) { throw "Native app exited: $($process.ExitCode)" }
        if ($startup.Elapsed.TotalSeconds -gt 30) { throw 'Native window was not created' }
    } while ($process.MainWindowHandle -eq 0 -or -not $process.MainWindowTitle.StartsWith('Sealed Heaven'))
    # Window creation precedes initial renderer/font setup. Do not send the
    # first QA key while winit is still initializing its input/render schedule.
    Start-Sleep -Seconds 8
    Snapshot 'startup'
    Assert-True ($last.tick -eq 0 -and $last.paused) 'Must start paused'
    Button 'Step'; Snapshot 'tick1-eat'
    Assert-True ($last.tick -eq 1 -and $last.inspector.Contains('Goal: Eat')) 'First eat goal missing'
    foreach ($pair in @(@(2,'Rest'),@(3,'Work'),@(4,'Haul'),@(5,'Return home'))) {
        if ($KeyboardOnly) { Post-AppKey $process.MainWindowHandle 0x09 0x0F }
        else {
            $marker=$last.marker_positions | Where-Object id -eq $pair[0] | Select-Object -First 1
            Post-AppClick $process.MainWindowHandle ([int]$marker.x) ([int]$marker.y)
        }
        Start-Sleep -Milliseconds 200
        Snapshot ('select-' + $pair[1])
        Assert-True ($last.selected -eq $pair[0] -and $last.inspector.Contains('Goal: ' + $pair[1])) 'Selection/goal mismatch'
    }
    Button 'Step'; Snapshot 'tick2-inflight'
    Assert-True ($last.tick -eq 2) 'Step must advance one tick'
    Button 'Save'; Snapshot 'saved-tick2'
    Assert-True ($last.status -eq 'status.saved') 'Save button failed'
    $savedHash=$last.hash
    Button 'Step'; Snapshot 'tick3-first'
    $tick3=$last.hash
    Button 'Load'; Snapshot 'loaded-tick2'
    Assert-True ($last.tick -eq 2 -and $last.paused -and $last.hash -eq $savedHash) 'Load must restore the exact paused checkpoint'
    Button 'Step'; Snapshot 'tick3-replayed'
    Assert-True ($last.hash -eq $tick3) 'Native load suffix mismatch'
    Button 'Step'; Snapshot 'tick4-complete'
    $stable=$last.hash
    Button 'Indonesian'; Snapshot 'indonesian'
    Assert-True ($last.locale -eq 'Indonesian' -and $last.hash -eq $stable) 'Language mutated canonical state'
    Post-AppKey $process.MainWindowHandle 0x09 0x0F
    Post-AppKey $process.MainWindowHandle 0x44 0x20
    Post-AppKey $process.MainWindowHandle 0x51 0x10
    Snapshot 'selection-pan-zoom'
    Assert-True ($last.selected -eq 6 -and $last.hash -eq $stable) 'Presentation controls changed simulation'
    Post-AppKey $process.MainWindowHandle 0x43 0x2E
    Post-AppKey $process.MainWindowHandle 0x23 0x4F
    Snapshot 'camera-reset-event-log'
    Assert-True ($last.hash -eq $stable) 'Camera reset/scroll changed simulation'
    $process.Refresh()
    $cpuBefore=$process.CPU
    $sample=[Diagnostics.Stopwatch]::StartNew()
    Button 'Toggle'; Start-Sleep -Seconds 10; Button 'Toggle'
    $sample.Stop()
    $process.Refresh()
    $cpuSeconds=$process.CPU-$cpuBefore
    $performance=@{
        seconds=$sample.Elapsed.TotalSeconds; process_cpu_seconds=$cpuSeconds
        cpu_one_core_equivalent_percent=(100*$cpuSeconds/$sample.Elapsed.TotalSeconds)
        working_set_bytes=$process.WorkingSet64; peak_working_set_bytes=$process.PeakWorkingSet64
        build='debug'; actors=20; renderer=$last.renderer
        cpu=(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
        physical_memory_bytes=(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
        limitation='Short native QA sample including input overhead, not an FPS benchmark or 30-day/R2 performance gate.'
    }
    $performance | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $EvidenceDirectory 'performance.json')
    Snapshot 'resume-then-pause'
    Assert-True ($last.tick -gt 4 -and $last.paused) 'Run/pause buttons failed'
    $pausedHash=$last.hash
    Start-Sleep -Seconds 1
    Snapshot 'paused-stable'
    Assert-True ($last.hash -eq $pausedHash) 'Paused native window advanced'
    Button 'Reset'; Snapshot 'reset-demo'
    Assert-True ($last.tick -eq 0 -and $last.paused) 'Reset button failed'
    Button 'English'; Snapshot 'english-restored'
    Assert-True ($last.locale -eq 'English') 'English button failed'
    $notes.Add("renderer=$($last.renderer)")
    $notes.Add('Native window messages exercised actual winit mouse/keyboard input; PNGs come from the actual Bevy framebuffer, not a mock UI or desktop capture.')
    if ($KeyboardOnly) { $notes.Add('PARTIAL PASS: native keyboard controls/render verified. Mouse selection and clickable toolbar remain unverified; this is not C04-T07 or milestone PASS.') }
    else { $notes.Add('PASS: native interaction smoke. Independent review and automated checks remain separate.') }
} finally {
    $notes | Set-Content -Encoding UTF8 (Join-Path $EvidenceDirectory 'smoke-log.txt')
    if (-not $process.HasExited) { $process.CloseMainWindow() | Out-Null }
    Write-Output "Native evidence: $EvidenceDirectory"
}
