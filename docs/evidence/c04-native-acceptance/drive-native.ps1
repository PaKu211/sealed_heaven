# Drives the already-running C0.4 window with real cursor and keyboard events.
# F12 evidence is posted to that window only. This script refuses to continue
# if the game is not the foreground window before an input event.
$ErrorActionPreference = 'Stop'
$evidence = Split-Path -Parent $MyInvocation.MyCommand.Path
$process = Get-Process -Id (Get-Content (Join-Path $evidence 'pid.txt'))
$process.Refresh()
$script:handle = $process.MainWindowHandle
$script:next = @(Get-ChildItem (Join-Path $evidence 'native-*.json') -ErrorAction SilentlyContinue).Count + 1
$script:last = $null
$notes = New-Object System.Collections.Generic.List[string]
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DriveIn {
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr id);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  public struct POINT { public int X, Y; }
}
'@
function Assert-True($condition, [string]$message) { if (-not $condition) { throw $message } }
function Focus-Game {
  if ([DriveIn]::GetForegroundWindow() -eq $script:handle) { return }
  [DriveIn]::SetForegroundWindow($script:handle) | Out-Null
  if ([DriveIn]::GetForegroundWindow() -ne $script:handle) {
    $thread = [DriveIn]::GetCurrentThreadId()
    $foreground = [DriveIn]::GetWindowThreadProcessId([DriveIn]::GetForegroundWindow(), [IntPtr]::Zero)
    $attached = [DriveIn]::AttachThreadInput($thread, $foreground, $true)
    try { [DriveIn]::SetForegroundWindow($script:handle) | Out-Null }
    finally { if ($attached) { [DriveIn]::AttachThreadInput($thread, $foreground, $false) | Out-Null } }
  }
  Assert-True ([DriveIn]::GetForegroundWindow() -eq $script:handle) 'Game window lost foreground; refusing input that could hit another application'
}
function Click-Client([int]$x, [int]$y) {
  Focus-Game
  $point = New-Object DriveIn+POINT
  $point.X = $x; $point.Y = $y
  Assert-True ([DriveIn]::ClientToScreen($script:handle, [ref]$point)) 'ClientToScreen failed'
  [DriveIn]::SetCursorPos($point.X, $point.Y) | Out-Null
  Start-Sleep -Milliseconds 80
  [DriveIn]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 50
  [DriveIn]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 250
}
function Click-Control([string]$control) {
  $button = $script:last.buttons | Where-Object control -eq $control | Select-Object -First 1
  Assert-True ($null -ne $button) "Missing button $control"
  Click-Client ([int]$button.x) ([int]$button.y)
}
function Tap-Key([byte]$vk, [byte]$scan, [uint32]$flags = 0) {
  Focus-Game
  [DriveIn]::keybd_event($vk, $scan, $flags, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [DriveIn]::keybd_event($vk, $scan, (2 -bor $flags), [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 180
}
function Hold-Key([byte]$vk, [byte]$scan, [int]$milliseconds) {
  Focus-Game
  [DriveIn]::keybd_event($vk, $scan, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds $milliseconds
  [DriveIn]::keybd_event($vk, $scan, 2, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 180
}
function Snapshot([string]$name) {
  $index = $script:next
  $script:next++
  $base = Join-Path $evidence ('native-{0:000}' -f $index)
  [DriveIn]::PostMessage($script:handle, 0x100, [IntPtr]0x7B, [IntPtr](1 -bor (0x58 -shl 16))) | Out-Null
  Start-Sleep -Milliseconds 60
  [DriveIn]::PostMessage($script:handle, 0x101, [IntPtr]0x7B, [IntPtr]([int64]0xC0000001L -bor (0x58 -shl 16))) | Out-Null
  $clock = [Diagnostics.Stopwatch]::StartNew()
  while ($true) {
    if ($clock.Elapsed.TotalSeconds -gt 15) { throw "Capture timed out: $name" }
    if ((Test-Path "$base.json") -and (Test-Path "$base.png")) {
      try {
        $bytes = [IO.File]::ReadAllBytes("$base.png")
        if ($bytes.Length -ge 24 -and [Text.Encoding]::ASCII.GetString($bytes, $bytes.Length - 8, 4) -eq 'IEND') { break }
      } catch [IO.IOException] { }
    }
    Start-Sleep -Milliseconds 100
  }
  $image = [System.Drawing.Image]::FromFile("$base.png")
  $width = $image.Width; $height = $image.Height
  $bitmap = New-Object System.Drawing.Bitmap $image
  $image.Dispose()
  $colors = New-Object 'System.Collections.Generic.HashSet[int]'
  for ($y = 0; $y -lt $height; $y += 40) {
    for ($x = 0; $x -lt $width; $x += 40) { [void]$colors.Add($bitmap.GetPixel($x, $y).ToArgb()) }
  }
  $bitmap.Dispose()
  Assert-True ($width -ge 1200 -and $height -ge 700) "Framebuffer too small: $name ${width}x${height}"
  Assert-True ($colors.Count -ge 8) "Framebuffer looks blank: $name"
  $script:last = Get-Content -Raw -Encoding UTF8 "$base.json" | ConvertFrom-Json
  Assert-True ($script:last.actors -eq 20) "Expected 20 NPCs at $name"
  Assert-True ($script:last.diagnostic -eq '') "Diagnostic at ${name}: $($script:last.diagnostic)"
  $notes.Add("$name => native-$('{0:000}' -f $index) tick=$($script:last.tick) paused=$($script:last.paused) selected=$($script:last.selected) locale=$($script:last.locale) hash=$($script:last.hash) digest=$($script:last.digest) status=$($script:last.status)")
}
function Goal-Line { ($script:last.inspector -split "`n" | Where-Object { $_ -like 'Goal:*' -or $_ -like 'Tujuan:*' } | Select-Object -First 1) }
function Site-Line { ($script:last.inspector -split "`n" | Where-Object { $_ -match 'Site:|Lokasi:' } | Select-Object -First 1) }
try {
  Snapshot 'startup'
  if (-not ($script:last.tick -eq 0 -and $script:last.paused)) {
    Click-Control 'Reset'
    Snapshot 'reset-before-script'
  }
  Assert-True ($script:last.tick -eq 0 -and $script:last.paused -and $script:last.actors -eq 20) 'Fresh demo did not start paused at tick 0'
  Click-Control 'Step'
  Snapshot 'tick1-eat'
  Assert-True ($script:last.tick -eq 1 -and $script:last.paused -and $script:last.inspector.Contains('Goal: Eat [1]')) 'Tick-1 eat click failed'
  $expected = @{
    1 = 'Goal: Eat [1]'; 2 = 'Goal: Rest [2]'; 3 = 'Goal: Work [3]'
    4 = 'Goal: Haul [4]'; 5 = 'Goal: Return home [5]'
  }
  $stages = @{ 1 = 'Executing [4]'; 2 = 'Travelling [3]'; 3 = 'Executing [4]'; 4 = 'Travelling [3]'; 5 = 'Travelling [3]' }
  foreach ($id in 1..5) {
    $marker = $script:last.marker_positions | Where-Object id -eq $id | Select-Object -First 1
    Click-Client ([int]$marker.x) ([int]$marker.y)
    Snapshot "mouse-npc-$id"
    Assert-True ($script:last.tick -eq 1 -and $script:last.selected -eq $id) "Mouse did not select NPC $id"
    Assert-True ($script:last.inspector.Contains($expected[$id])) "NPC $id goal mismatch: $(Goal-Line)"
    Assert-True ($script:last.inspector.Contains($stages[$id])) "NPC $id stage mismatch"
    Assert-True ($script:last.inspector.Contains('Canonical position') -and $script:last.inspector.Contains('Stored need channels') -and $script:last.inspector.Contains('Actor inventory')) "NPC $id inspector missing authoritative fields"
    $notes.Add("npc$id $(Goal-Line) / $(Site-Line)")
  }
  $tick1Hash = $script:last.hash
  Tap-Key 0x09 0x0F
  Snapshot 'tab-after-npc5'
  Assert-True ($script:last.selected -eq 6 -and $script:last.hash -eq $tick1Hash) 'Tab selection changed the checkpoint or missed NPC 6'
  Click-Control 'Step'; Snapshot 'tick2'
  Assert-True ($script:last.tick -eq 2 -and $script:last.paused) 'Single step did not advance exactly one tick'
  $notes.Add("tick2-site $(Site-Line)")
  Click-Control 'Save'; Snapshot 'saved-tick2'
  Assert-True ($script:last.status -eq 'status.saved' -and $script:last.tick -eq 2) 'Save click failed'
  $savedHash = $script:last.hash; $savedDigest = $script:last.digest
  Click-Control 'Step'; Snapshot 'tick3-first'
  Assert-True ($script:last.tick -eq 3 -and $script:last.paused) 'Second step was not exactly one tick'
  $tick3Hash = $script:last.hash; $tick3Digest = $script:last.digest
  Click-Control 'Load'; Snapshot 'loaded-tick2'
  Assert-True ($script:last.tick -eq 2 -and $script:last.paused -and $script:last.hash -eq $savedHash -and $script:last.digest -eq $savedDigest) 'Load did not restore the paused tick-2 checkpoint'
  Click-Control 'Step'; Snapshot 'tick3-replayed'
  Assert-True ($script:last.tick -eq 3 -and $script:last.hash -eq $tick3Hash -and $script:last.digest -eq $tick3Digest) 'Continued tick-3 hash diverged'
  Click-Control 'Step'; Snapshot 'tick4'
  Assert-True ($script:last.tick -eq 4 -and $script:last.paused) 'Step to tick 4 failed'
  $stableHash = $script:last.hash; $stableDigest = $script:last.digest
  foreach ($pair in @(@(1,1), @(2,2), @(3,1), @(4,2), @(5,2))) {
    $marker = $script:last.marker_positions | Where-Object id -eq $pair[0] | Select-Object -First 1
    Click-Client ([int]$marker.x) ([int]$marker.y)
    Snapshot "arrival-npc-$($pair[0])"
    Assert-True ($script:last.selected -eq $pair[0] -and $script:last.hash -eq $stableHash) "Arrival selection changed simulation for NPC $($pair[0])"
    Assert-True ((Site-Line) -match "Site: $($pair[1])") "NPC $($pair[0]) expected site $($pair[1]): $(Site-Line)"
  }
  Assert-True ($script:last.inspector -match 'ID 101 · Definition 4 · Quantity 4' -and $script:last.inspector -match 'ID 102 · Definition 5 · Quantity 4') 'Durable stocks 101/102 changed'
  $stockText = $script:last.inspector
  Assert-True ($stockText.Contains('Site stockpile') -and $stockText.Contains('Owner:')) 'Ownership lines missing'
  $ids = [regex]::Matches($script:last.inspector, 'ID (\d+) · Tick (\d+) · NPC') | ForEach-Object { [int]$_.Groups[1].Value }
  Assert-True ($ids.Count -ge 2) 'Ordered event log missing'
  $ordered = $true
  for ($i = 1; $i -lt $ids.Count; $i++) { if ($ids[$i] -le $ids[$i-1]) { $ordered = $false } }
  Assert-True $ordered 'Event log is not in ascending kernel order'
  $notes.Add("event-ids=$($ids[0])..$($ids[-1]) count=$($ids.Count)")
  Click-Client 1100 420
  [DriveIn]::mouse_event(0x0800, 0, 0, [uint32](-360), [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 200
  Tap-Key 0x22 0x51 1
  Tap-Key 0x23 0x4F 1
  Snapshot 'event-log-scrolled'
  Assert-True ($script:last.hash -eq $stableHash -and $script:last.digest -eq $stableDigest) 'Inspector scroll changed the checkpoint'
  Click-Control 'English'; Snapshot 'english-click'
  Assert-True ($script:last.locale -eq 'English' -and $script:last.hash -eq $stableHash -and $script:last.digest -eq $stableDigest) 'English click changed canonical state'
  Click-Control 'Indonesian'; Snapshot 'indonesian'
  Assert-True ($script:last.locale -eq 'Indonesian' -and $script:last.hash -eq $stableHash -and $script:last.digest -eq $stableDigest) 'Indonesian click changed canonical state'
  Assert-True ($script:last.inspector.Contains('Pemeriksa dunia') -and $script:last.inspector.Contains('Tujuan:')) 'Indonesian labels did not replace the inspector'
  $before = @{}
  foreach ($marker in $script:last.marker_positions) { $before[[int]$marker.id] = @([double]$marker.x, [double]$marker.y) }
  Hold-Key 0x44 0x20 500
  Snapshot 'pan-right'
  $panned = $script:last.marker_positions | Where-Object id -eq 1
  Assert-True ([Math]::Abs($panned.x - $before[1][0]) -gt 30 -and $script:last.hash -eq $stableHash) 'Camera pan did not move the view or changed the checkpoint'
  $gapBefore = [Math]::Abs($before[2][0] - $before[1][0])
  Hold-Key 0x51 0x10 600
  Snapshot 'zoom-in'
  $m1 = $script:last.marker_positions | Where-Object id -eq 1
  $m2 = $script:last.marker_positions | Where-Object id -eq 2
  Assert-True ([Math]::Abs($m2.x - $m1.x) -gt ($gapBefore + 5) -and $script:last.hash -eq $stableHash) 'Camera zoom did not change marker spacing or changed the checkpoint'
  Tap-Key 0x43 0x2E
  Snapshot 'camera-reset'
  $reset = $script:last.marker_positions | Where-Object id -eq 1
  Assert-True ([Math]::Abs($reset.x - $before[1][0]) -lt 3 -and [Math]::Abs($reset.y - $before[1][1]) -lt 3 -and $script:last.hash -eq $stableHash) 'Camera reset did not restore the view'
  $process.Refresh(); $cpuBefore = $process.CPU
  $sample = [Diagnostics.Stopwatch]::StartNew()
  Click-Control 'Toggle'
  Start-Sleep -Seconds 10
  Click-Control 'Toggle'
  $sample.Stop()
  $process.Refresh()
  Snapshot 'resume-then-pause'
  Assert-True ($script:last.paused -and $script:last.tick -ge 12) "Resume/pause did not run the twenty NPCs: tick=$($script:last.tick)"
  $runningTick = $script:last.tick
  $pausedHash = $script:last.hash
  Start-Sleep -Seconds 1
  Snapshot 'paused-stable'
  Assert-True ($script:last.paused -and $script:last.tick -eq $runningTick -and $script:last.hash -eq $pausedHash) 'Paused simulation advanced'
  Click-Control 'Reset'; Snapshot 'reset-demo'
  Assert-True ($script:last.tick -eq 0 -and $script:last.paused -and $script:last.status -eq 'status.reset') 'Reset click did not return to paused tick 0'
  Click-Control 'English'; Snapshot 'english-restored'
  Assert-True ($script:last.locale -eq 'English' -and $script:last.tick -eq 0 -and $script:last.paused) 'English restore failed'
  $cpuSeconds = $process.CPU - $cpuBefore
  $performance = [ordered]@{
    seconds = [Math]::Round($sample.Elapsed.TotalSeconds, 3)
    process_cpu_seconds = [Math]::Round($cpuSeconds, 3)
    cpu_one_core_equivalent_percent = [Math]::Round(100 * $cpuSeconds / $sample.Elapsed.TotalSeconds, 1)
    working_set_bytes = $process.WorkingSet64
    peak_working_set_bytes = $process.PeakWorkingSet64
    build = 'debug'
    actors = 20
    renderer = $script:last.renderer
    cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
    physical_memory_bytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
    sampling = 'Process.CPU and working set around a real foreground Resume click, 10 second run, and Pause click. Includes input and capture overhead.'
    limitation = 'Short running twenty-NPC debug observation. Not an FPS benchmark and not the deferred 30-day or R2 performance gate.'
  }
  $performance | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $evidence 'performance.json')
  $notes.Add("renderer=$($script:last.renderer)")
  $notes.Add('PASS: real foreground mouse clicks hit every toolbar control and NPC markers 1-5; Tab, pan, zoom, camera reset, wheel and PageDown/End were real keyboard or mouse events. PNGs are the Bevy framebuffer.')
} finally {
  $notes | Set-Content -Encoding UTF8 (Join-Path $evidence 'smoke-log.txt')
}
