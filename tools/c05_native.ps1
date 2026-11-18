# Actual Windows foreground gameplay, never a scripted state mutation or mock UI.
param(
    [Parameter(Mandatory=$true)][string]$EvidenceDirectory,
    [ValidateSet('debug','release')][string]$Profile='release',
    [switch]$RenderOnly
)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$dir=[IO.Path]::GetFullPath($EvidenceDirectory)
if(Test-Path $dir) {throw 'Use a fresh evidence directory to avoid accepting stale captures'}
New-Item -ItemType Directory $dir | Out-Null
$exe=Join-Path $root "apps/sealed_playable/target/$Profile/sealed_agency.exe"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class C05Input {
  public struct POINT { public int X,Y; }
  [StructLayout(LayoutKind.Sequential)] public struct MSG { public IntPtr hwnd; public uint message; public UIntPtr wParam; public IntPtr lParam; public uint time; public POINT point; public uint privateData; }
  public delegate bool WindowVisitor(IntPtr hwnd,IntPtr data);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int c);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,IntPtr id);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from,uint to,bool attach);
  [DllImport("user32.dll")] public static extern bool PeekMessage(out MSG msg,IntPtr hwnd,uint min,uint max,uint remove);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr SetActiveWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr SetFocus(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool EnumWindows(WindowVisitor callback,IntPtr data);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd,System.Text.StringBuilder text,int max);
  public static IntPtr FindGame(uint pid) {
    IntPtr found=IntPtr.Zero;
    EnumWindows((hwnd,data)=>{
      uint owner;
      GetWindowThreadProcessId(hwnd,out owner);
      var text=new System.Text.StringBuilder(256);
      GetWindowText(hwnd,text,text.Capacity);
      if(owner==pid && text.ToString().Contains("C0.5")) {found=hwnd;return false;}
      return true;
    },IntPtr.Zero);
    return found;
  }
  [DllImport("user32.dll",EntryPoint="GetWindowThreadProcessId")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint id);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint x,uint y,uint data,UIntPtr extra);
  [DllImport("user32.dll")] public static extern void keybd_event(byte k,byte s,uint f,UIntPtr extra);
}
'@
$oldLocal=$env:LOCALAPPDATA
$oldEvidence=$env:SEALED_HEAVEN_EVIDENCE_DIR
$env:LOCALAPPDATA=$dir
$env:SEALED_HEAVEN_EVIDENCE_DIR=$dir
try {
    $process=Start-Process -FilePath $exe -PassThru -WorkingDirectory $root -RedirectStandardOutput "$dir/stdout.log" -RedirectStandardError "$dir/stderr.log"
    $null=$process.Handle
} finally {$env:LOCALAPPDATA=$oldLocal;$env:SEALED_HEAVEN_EVIDENCE_DIR=$oldEvidence}
$process.Id|Set-Content "$dir/pid.txt"
$notes=New-Object 'System.Collections.Generic.List[string]'
$script:index=0;$script:last=$null;$script:handle=[IntPtr]::Zero
function Assert-True($condition,[string]$message) {if(-not $condition) {throw $message}}
function Focus-Game {
    $process.Refresh()
    if($process.HasExited) {throw "Game exited: $($process.ExitCode)"}
    if([C05Input]::IsIconic($script:handle)) {[C05Input]::ShowWindowAsync($script:handle,9)|Out-Null;Start-Sleep -Milliseconds 600}
    if([C05Input]::GetForegroundWindow() -eq $script:handle) {return}
    [C05Input]::SetForegroundWindow($script:handle)|Out-Null
    if([C05Input]::GetForegroundWindow() -ne $script:handle) {
        $message=New-Object C05Input+MSG
        [C05Input]::PeekMessage([ref]$message,[IntPtr]::Zero,0,0,0)|Out-Null
        $thread=[C05Input]::GetCurrentThreadId()
        $foreground=[C05Input]::GetWindowThreadProcessId([C05Input]::GetForegroundWindow(),[IntPtr]::Zero)
        $targetThread=[C05Input]::GetWindowThreadProcessId($script:handle,[IntPtr]::Zero)
        $attached=[C05Input]::AttachThreadInput($thread,$foreground,$true)
        $targetAttached=if($targetThread -ne $foreground){[C05Input]::AttachThreadInput($thread,$targetThread,$true)}else{$false}
        try {
            [C05Input]::BringWindowToTop($script:handle)|Out-Null
            [C05Input]::SetActiveWindow($script:handle)|Out-Null
            [C05Input]::SetForegroundWindow($script:handle)|Out-Null
            [C05Input]::SetFocus($script:handle)|Out-Null
        } finally {
            if($targetAttached){[C05Input]::AttachThreadInput($thread,$targetThread,$false)|Out-Null}
            if($attached) {[C05Input]::AttachThreadInput($thread,$foreground,$false)|Out-Null}
        }
    }
    Start-Sleep -Milliseconds 100
    Assert-True ([C05Input]::GetForegroundWindow() -eq $script:handle) 'Cannot focus game safely; refusing unrelated-window input'
}
function Key([byte]$vk,[byte]$scan,[uint32]$flags=0) {
    Focus-Game
    [C05Input]::keybd_event($vk,$scan,$flags,[UIntPtr]::Zero)
    try {Start-Sleep -Milliseconds 90}
    finally {[C05Input]::keybd_event($vk,$scan,($flags -bor 2),[UIntPtr]::Zero)}
    Start-Sleep -Milliseconds 180
}
function Click([int]$x,[int]$y) {
    Focus-Game
    $point=New-Object C05Input+POINT;$point.X=$x;$point.Y=$y
    Assert-True ([C05Input]::ClientToScreen($script:handle,[ref]$point)) 'Client coordinates unavailable'
    [C05Input]::SetCursorPos($point.X,$point.Y)|Out-Null
    $packed=($y -shl 16) -bor ($x -band 0xFFFF)
    [C05Input]::PostMessage($script:handle,0x200,[IntPtr]::Zero,[IntPtr]$packed)|Out-Null
    Start-Sleep -Milliseconds 100
    Assert-True ([C05Input]::GetForegroundWindow() -eq $script:handle) 'Game lost focus before click'
    [C05Input]::mouse_event(2,0,0,0,[UIntPtr]::Zero)
    try {Start-Sleep -Milliseconds 90}
    finally {[C05Input]::mouse_event(4,0,0,0,[UIntPtr]::Zero)}
    Start-Sleep -Milliseconds 200
}
function Button([string]$name) {
    $button=$script:last.buttons|Where-Object control -eq $name|Select-Object -First 1
    Assert-True ($null -ne $button) "Missing button: $name"
    Click ([int]$button.x) ([int]$button.y)
}
function Target([string]$name) {
    $target=$script:last.targets|Where-Object target -eq $name|Select-Object -First 1
    Assert-True ($null -ne $target) "Missing visible target: $name"
    Click ([int]$target.x) ([int]$target.y)
}
function Snapshot([string]$name) {
    if(-not $RenderOnly) {Focus-Game}
    $script:index++
    $base=Join-Path $dir ('c05-{0:000}' -f $script:index)
    if($RenderOnly) {
        [C05Input]::PostMessage($script:handle,0x100,[IntPtr]0x7B,[IntPtr](1 -bor (0x58 -shl 16)))|Out-Null
        Start-Sleep -Milliseconds 150
        [C05Input]::PostMessage($script:handle,0x101,[IntPtr]0x7B,[IntPtr]([int64]0xC0000001L -bor (0x58 -shl 16)))|Out-Null
    } else {Key 0x7B 0x58}
    $watch=[Diagnostics.Stopwatch]::StartNew()
    while($true) {
        if($watch.Elapsed.TotalSeconds -gt 20) {throw "Capture timed out: $name"}
        if((Test-Path "$base.json") -and (Test-Path "$base.png")) {
            try {
                $png=[IO.File]::ReadAllBytes("$base.png")
                if($png.Length -ge 24 -and [Text.Encoding]::ASCII.GetString($png,$png.Length-8,4) -eq 'IEND') {break}
            } catch [IO.IOException] {}
        }
        Start-Sleep -Milliseconds 100
    }
    $width=$png[16]*16777216+$png[17]*65536+$png[18]*256+$png[19]
    $height=$png[20]*16777216+$png[21]*65536+$png[22]*256+$png[23]
    Assert-True ($width -ge 1200 -and $height -ge 700) "Minimized/small framebuffer: ${width}x${height}"
    $script:last=Get-Content -Raw -Encoding UTF8 "$base.json"|ConvertFrom-Json
    Assert-True ($last.diagnostic -eq '') "Game diagnostic: $($last.diagnostic)"
    Assert-True ($last.npc_count -eq 20 -and $last.player_id -eq 21) 'Actor identity/count mismatch'
    Assert-True ($last.source+$last.player.herbs+$last.delivered -eq 3) 'Herb conservation failed'
    $notes.Add("$name => $(Split-Path $base -Leaf) tick=$($last.tick) player=($($last.player.x),$($last.player.y)) inventory=$($last.player.herbs) source=$($last.source) shrine=$($last.delivered) requested=$($last.requested) completed=$($last.completed) paused=$($last.paused) queued=$($last.queued) selected=$($last.selected) locale=$($last.locale) hash=$($last.hash)")
}
function Step-Key([byte]$key,[byte]$scan) {Key $key $scan;Key 0x4E 0x31}
function Interact-Step([string]$name) {
    $startingTick=[int64]$script:last.tick
    Key 0x45 0x12
    Snapshot "$name-queued"
    Assert-True ($last.tick -eq $startingTick -and $last.queued -eq 1) "Native interaction did not visibly queue: $name"
    $queuedTick=[int64]$last.tick
    for($attempt=1;$attempt -le 3;$attempt++) {
        Key 0x4E 0x31
        Snapshot $(if($attempt -eq 1){$name}else{"$name-step-retry-$attempt"})
        if($last.queued -eq 0) {break}
    }
    Assert-True ($last.tick -eq $queuedTick+1 -and $last.queued -eq 0) "Native step did not commit exactly one queued interaction: $name"
}
try {
    $startup=[Diagnostics.Stopwatch]::StartNew()
    do {
        Start-Sleep -Milliseconds 250;$process.Refresh()
        if($process.HasExited) {throw "Native startup failed: $($process.ExitCode)"}
        if($startup.Elapsed.TotalSeconds -gt 40) {throw 'Native window unavailable'}
        $script:handle=[C05Input]::FindGame($process.Id)
    } while($script:handle -eq [IntPtr]::Zero)
    Start-Sleep -Seconds 6
    if($RenderOnly) {
        if([C05Input]::IsIconic($script:handle)){[C05Input]::ShowWindowAsync($script:handle,4)|Out-Null;Start-Sleep -Seconds 1}
        Snapshot 'render-only-startup-NOT-gameplay-PASS'
        $notes.Add('PARTIAL: framebuffer render only; no native gameplay inputs attempted. Does NOT close C05-T09.')
        return
    }
    Snapshot 'paused-entrance'
    Assert-True ($last.tick -eq 0 -and $last.paused -and $last.player.x -eq 0 -and $last.player.y -eq 4) 'Incorrect initial state'
    $initialHash=$last.hash
    Interact-Step 'out-of-range-rejected'
    Assert-True ($last.body.Contains('Target is not within one cell') -and -not $last.requested) 'Invalid interaction not visibly rejected'
    Step-Key 0x44 0x20;Step-Key 0x44 0x20;Step-Key 0x57 0x11
    Snapshot 'approached-yun'
    Assert-True ($last.player.x -eq 2 -and $last.player.y -eq 3) 'Exploration did not use canonical movement'
    Target 'Npc(1)';Snapshot 'mouse-selected-yun'
    Assert-True ($last.selected -eq 'Npc(1)') 'Mouse NPC selection failed'
    Interact-Step 'request-known'
    Assert-True $last.requested 'Mouse interact did not commit the nearby talk'
    Step-Key 0x53 0x1F
    foreach($i in 1..6) {Step-Key 0x44 0x20}
    Step-Key 0x57 0x11;Step-Key 0x57 0x11
    Snapshot 'east-of-gate-herb-patch'
    Assert-True ($last.player.x -eq 8 -and $last.player.y -eq 2) 'Exploration path did not reach patch'
    Key 0x09 0x0F;Snapshot 'tab-select-patch'
    Assert-True ($last.selected -eq 'HerbPatch') 'Tab did not select patch'
    Interact-Step 'herb-acquired'
    Assert-True ($last.player.herbs -eq 1 -and $last.source -eq 2) 'Acquisition not canonical'
    Key 0x74 0x3F;Snapshot 'saved-with-herb'
    $savedHash=$last.hash
    Step-Key 0x53 0x1F;Snapshot 'future-after-save'
    $futureHash=$last.hash
    Key 0x78 0x43;Snapshot 'loaded-with-herb'
    Assert-True ($last.hash -eq $savedHash -and $last.paused -and $last.player.herbs -eq 1) 'Load did not restore carried herb'
    Step-Key 0x53 0x1F;Snapshot 'future-replayed'
    Assert-True ($last.hash -eq $futureHash) 'Save/load suffix changed'
    Step-Key 0x53 0x1F
    foreach($i in 1..5) {Step-Key 0x41 0x1E}
    Snapshot 'near-delivery-shrine'
    Assert-True ($last.player.x -eq 3 -and $last.player.y -eq 4) 'Return path incorrect'
    Target 'Shrine';Snapshot 'mouse-selected-shrine'
    Assert-True ($last.selected -eq 'Shrine') 'Object selection failed'
    Interact-Step 'delivered'
    Assert-True ($last.completed -and $last.player.herbs -eq 0 -and $last.delivered -eq 1 -and $last.source -eq 2) 'Delivery not atomic/completed'
    Key 0x74 0x3F;Snapshot 'completion-saved';$completedHash=$last.hash
    Interact-Step 'repeat-delivery-rejected'
    Assert-True ($last.delivered -eq 1 -and $last.body.Contains('This delivery is already complete')) 'Duplicate delivery was not rejected'
    Key 0x78 0x43;Snapshot 'completion-loaded'
    Assert-True ($last.hash -eq $completedHash -and $last.completed -and $last.delivered -eq 1) 'Completion not persistent'
    Key 0x4C 0x26;Snapshot 'indonesian'
    Assert-True ($last.locale -eq 'Indonesian' -and $last.hash -eq $completedHash -and $last.body.Contains('Herba diserahkan')) 'Localization changed domain or missing translated result'
    Key 0x43 0x2E;Snapshot 'camera-centered'
    Assert-True ($last.hash -eq $completedHash) 'Camera changed canonical state'
    Key 0x4C 0x26;Snapshot 'english'
    Assert-True ($last.locale -eq 'English' -and $last.hash -eq $completedHash) 'English restore changed domain'
    $beforeTick=$last.tick;$beforeNpc=$last.npc_next_event
    Key 0x20 0x39;Snapshot 'running-start'
    $runStartTick=$last.tick
    $process.Refresh();$beforeCpu=$process.CPU;$beforeMemory=$process.WorkingSet64
    $timer=[Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds 10
    Focus-Game
    $timer.Stop();$process.Refresh()
    $cpu=$process.CPU-$beforeCpu;$afterMemory=$process.WorkingSet64
    Snapshot 'running-end'
    $endTick=$last.tick
    $perf=[ordered]@{
        profile=$Profile;seconds=$timer.Elapsed.TotalSeconds;ticks=$endTick-$runStartTick;nominal_hz=2
        observed_wall_tps=($endTick-$runStartTick)/$timer.Elapsed.TotalSeconds
        process_cpu_seconds=$cpu;cpu_one_core_percent=100*$cpu/$timer.Elapsed.TotalSeconds
        working_set_before_bytes=$beforeMemory;working_set_after_bytes=$afterMemory;peak_working_set_bytes=$process.PeakWorkingSet64
        renderer=$last.renderer;entities=$last.entities;frame_time=$last.frame_time;tick_compute=$last.tick_compute
        cpu=(Get-CimInstance Win32_Processor|Select-Object -First 1 -ExpandProperty Name)
        ram_bytes=(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
        scope='Foreground real-time client with 20 autonomous NPCs, paused-action loop already completed. Tick boundaries bracket F12 captures; wall sample excludes most capture wait but may include boundary/input latency. CPU includes rendering; frame_time is wall frame interval not isolated GPU work. Not R2/30-day.'
    }
    $perf|ConvertTo-Json -Depth 12|Set-Content -Encoding UTF8 "$dir/performance.json"
    Key 0x20 0x39;Snapshot 'paused-after-run'
    Assert-True ($last.paused -and $last.tick -gt $beforeTick -and $last.npc_next_event -gt $beforeNpc) 'Autonomous NPCs did not continue'
    $pausedHash=$last.hash
    Start-Sleep -Seconds 1
    Snapshot 'pause-held'
    Assert-True ($last.hash -eq $pausedHash) 'Pause changed the world'
    Key 0x52 0x13;Snapshot 'reset'
    Assert-True ($last.paused -and $last.tick -eq 0 -and $last.hash -eq $initialHash) 'Reset did not restore original aggregate'
    $notes.Add('PASS: actual foreground keyboard/mouse gameplay; request/gather/deliver, invalid feedback, replay/save/load, locale, pause/step, camera centering and live NPCs. Game framebuffer evidence; no state injection.')
} catch {
    $notes.Add("FAIL/BLOCKED: $($_.Exception.Message)")
    throw
} finally {
    $notes|Set-Content -Encoding UTF8 "$dir/gameplay-log.txt"
    Get-FileHash -Algorithm SHA256 $exe|Format-List|Out-String|Set-Content "$dir/binary-sha256.txt"
    if(-not $process.HasExited) {$process.CloseMainWindow()|Out-Null}
    Write-Output "C05 native evidence: $dir"
}
