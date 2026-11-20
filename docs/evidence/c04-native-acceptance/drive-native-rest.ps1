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
  Start-Sleep -Milliseconds 160
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
function Select-Npc([int]$id) {
  foreach ($nudge in @(@(0,0), @(4,0), @(-4,0), @(0,4), @(0,-4))) {
    $marker = $script:last.marker_positions | Where-Object id -eq $id | Select-Object -First 1
    Click-Client ([int]$marker.x + $nudge[0]) ([int]$marker.y + $nudge[1])
    if ($nudge[0] -eq 0 -and $nudge[1] -eq 0) { Snapshot "arrival-npc-$id" } else { Snapshot "arrival-npc-$id-nudge" }
    if ($script:last.selected -eq $id) { return }
  }
  throw "Mouse did not select NPC $id"
}
try {
  Snapshot 'zoom-q-confirmed'
  $stableHash = '8faa211de3f29bc79031093ed4bec3a18b241baa088ac38f4bdacae1422c088f'
  Assert-True ($script:last.tick -eq 4 -and $script:last.paused -and $script:last.hash -eq $stableHash) 'Zoom continuation left the tick-4 checkpoint'
  $m1 = $script:last.marker_positions | Where-Object id -eq 1
  $m2 = $script:last.marker_positions | Where-Object id -eq 2
  $gap = [Math]::Abs([double]$m2.x - [double]$m1.x)
  Hold-Key 0x45 0x12 600
  Snapshot 'zoom-e'
  $e1 = $script:last.marker_positions | Where-Object id -eq 1
  $e2 = $script:last.marker_positions | Where-Object id -eq 2
  $gapE = [Math]::Abs([double]$e2.x - [double]$e1.x)
  Assert-True ([Math]::Abs($gapE - $gap) -gt 20 -and $script:last.hash -eq $stableHash) "E zoom did not change spacing ($gap -> $gapE)"
  Tap-Key 0x43 0x2E
  Snapshot 'camera-reset'
  $reset = $script:last.marker_positions | Where-Object id -eq 1
  Assert-True ([Math]::Abs($reset.x - 178.0) -lt 3 -and [Math]::Abs($reset.y - 338.5) -lt 3 -and $script:last.hash -eq $stableHash) "Camera reset marker was $($reset.x),$($reset.y)"
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
  $notes | Add-Content -Encoding UTF8 (Join-Path $evidence 'smoke-log.txt')
}
