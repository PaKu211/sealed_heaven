$ErrorActionPreference = 'Stop'
$evidence = Split-Path -Parent $MyInvocation.MyCommand.Path
$process = Get-Process -Id (Get-Content (Join-Path $evidence 'pid.txt'))
$process.Refresh()
$script:handle = $process.MainWindowHandle
$script:next = @(Get-ChildItem (Join-Path $evidence 'native-*.json')).Count + 1
$script:last = $null
$notes = New-Object System.Collections.Generic.List[string]
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class EndIn {
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr id);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool on);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint data, UIntPtr extra);
  public struct POINT { public int X, Y; }
}
'@
function Assert-True($condition, [string]$message) { if (-not $condition) { throw $message } }
function Focus-Game {
  if ([EndIn]::GetForegroundWindow() -eq $script:handle) { return }
  [EndIn]::SetForegroundWindow($script:handle) | Out-Null
  if ([EndIn]::GetForegroundWindow() -ne $script:handle) {
    $thread = [EndIn]::GetCurrentThreadId()
    $owner = [EndIn]::GetWindowThreadProcessId([EndIn]::GetForegroundWindow(), [IntPtr]::Zero)
    $attached = [EndIn]::AttachThreadInput($thread, $owner, $true)
    try { [EndIn]::SetForegroundWindow($script:handle) | Out-Null }
    finally { if ($attached) { [EndIn]::AttachThreadInput($thread, $owner, $false) | Out-Null } }
  }
  Assert-True ([EndIn]::GetForegroundWindow() -eq $script:handle) 'Game window lost foreground'
}
function Click-Control([string]$control) {
  $button = $script:last.buttons | Where-Object control -eq $control | Select-Object -First 1
  Assert-True ($null -ne $button) "Missing $control"
  Focus-Game
  $point = New-Object EndIn+POINT
  $point.X = [int][Math]::Round($button.x); $point.Y = [int][Math]::Round($button.y)
  Assert-True ([EndIn]::ClientToScreen($script:handle, [ref]$point)) 'ClientToScreen failed'
  [EndIn]::SetCursorPos($point.X, $point.Y) | Out-Null
  Start-Sleep -Milliseconds 150
  [EndIn]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 50
  [EndIn]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 250
}
function Snapshot([string]$name) {
  $index = $script:next; $script:next++
  $base = Join-Path $evidence ('native-{0:000}' -f $index)
  [EndIn]::PostMessage($script:handle, 0x100, [IntPtr]0x7B, [IntPtr](1 -bor (0x58 -shl 16))) | Out-Null
  Start-Sleep -Milliseconds 40
  [EndIn]::PostMessage($script:handle, 0x101, [IntPtr]0x7B, [IntPtr]([int64]0xC0000001L -bor (0x58 -shl 16))) | Out-Null
  $clock = [Diagnostics.Stopwatch]::StartNew()
  while ($true) {
    if ($clock.Elapsed.TotalSeconds -gt 15) { throw "Capture timed out: $name" }
    if ((Test-Path "$base.json") -and (Test-Path "$base.png")) {
      try {
        $bytes = [IO.File]::ReadAllBytes("$base.png")
        if ($bytes.Length -ge 24 -and [Text.Encoding]::ASCII.GetString($bytes, $bytes.Length - 8, 4) -eq 'IEND') { break }
      } catch [IO.IOException] { }
    }
    Start-Sleep -Milliseconds 80
  }
  $script:last = Get-Content -Raw -Encoding UTF8 "$base.json" | ConvertFrom-Json
  $notes.Add("$name => native-$('{0:000}' -f $index) tick=$($script:last.tick) paused=$($script:last.paused) locale=$($script:last.locale) hash=$($script:last.hash) status=$($script:last.status)")
}
try {
  Snapshot 'pause-confirmed'
  Assert-True ($script:last.paused) 'Pause click was not held'
  $heldTick = $script:last.tick; $heldHash = $script:last.hash
  Start-Sleep -Seconds 1
  Snapshot 'pause-held'
  Assert-True ($script:last.paused -and $script:last.tick -eq $heldTick -and $script:last.hash -eq $heldHash) 'Paused simulation advanced'
  Click-Control 'Toggle'
  Snapshot 'resumed-for-sample'
  Assert-True (-not $script:last.paused) 'Resume click failed'
  $startTick = $script:last.tick
  $process.Refresh(); $cpuBefore = $process.CPU; $wsBefore = $process.WorkingSet64
  $sample = [Diagnostics.Stopwatch]::StartNew()
  while ($sample.Elapsed.TotalSeconds -lt 10) { Focus-Game; Start-Sleep -Milliseconds 100 }
  $process.Refresh()
  Snapshot 'sample-end'
  $advanced = $script:last.tick - $startTick
  Assert-True ((-not $script:last.paused) -and $advanced -ge 12 -and $advanced -le 30) "Sample advanced $advanced ticks"
  Click-Control 'Toggle'
  Snapshot 'sample-paused'
  $pausedTick = $script:last.tick; $pausedHash = $script:last.hash
  Assert-True ($script:last.paused) 'Second pause click failed'
  Start-Sleep -Seconds 1
  Snapshot 'sample-paused-held'
  Assert-True ($script:last.tick -eq $pausedTick -and $script:last.hash -eq $pausedHash) 'Second pause did not hold'
  Click-Control 'Reset'
  Snapshot 'reset-demo'
  Assert-True ($script:last.tick -eq 0 -and $script:last.paused -and $script:last.status -eq 'status.reset') "Reset failed: tick=$($script:last.tick) status=$($script:last.status)"
  $resetHash = $script:last.hash
  Click-Control 'English'
  Snapshot 'english-restored'
  Assert-True ($script:last.locale -eq 'English' -and $script:last.tick -eq 0 -and $script:last.paused -and $script:last.hash -eq $resetHash) 'English restore changed tick 0'
  Assert-True ($script:last.inspector.Contains('Canonical position') -and $script:last.inspector.Contains('Goal: None')) 'Reset inspector is not the English tick-0 state'
  $cpu = $process.CPU - $cpuBefore
  [ordered]@{
    seconds = [Math]::Round($sample.Elapsed.TotalSeconds, 3)
    ticks_advanced = $advanced
    observed_ticks_per_second = [Math]::Round($advanced / $sample.Elapsed.TotalSeconds, 2)
    nominal_tick_hz = 2
    process_cpu_seconds = [Math]::Round($cpu, 3)
    cpu_one_core_equivalent_percent = [Math]::Round(100 * $cpu / $sample.Elapsed.TotalSeconds, 1)
    working_set_bytes_before = $wsBefore
    working_set_bytes_after = $process.WorkingSet64
    peak_working_set_bytes = $process.PeakWorkingSet64
    build = 'debug'
    actors = 20
    renderer = $script:last.renderer
    cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
    physical_memory_bytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
    sampling = 'The window stayed in the foreground. Process.CPU covers the whole debug process for the measured run, including rendering.'
    limitation = 'Short twenty-NPC debug observation after an earlier unfocused interval had already been caught up at no more than eight ticks per frame. Not an FPS benchmark and not the deferred 30-day or R2 gate.'
  } | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $evidence 'performance.json')
  $notes.Add('PASS finish: pause hold, focused 10-second run, pause hold, reset to tick 0, English restored.')
} finally {
  $notes | Add-Content -Encoding UTF8 (Join-Path $evidence 'smoke-log.txt')
}
