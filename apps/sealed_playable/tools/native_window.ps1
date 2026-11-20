# Native UI test helpers; target only the process/window created for this app.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SealedWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hwnd, int command);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr id);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
'@
function Prepare-AppWindow([IntPtr]$handle, [switch]$Foreground) {
    # Winit's mouse tracking requires a real cursor inside the app. Activate only
    # the launched target, verify foreground ownership, and never send global keys.
    if ([SealedWindow]::IsIconic($handle)) {
        [SealedWindow]::ShowWindowAsync($handle,4) | Out-Null
        Start-Sleep -Milliseconds 750
    }
    if (-not $Foreground) { return }
    [SealedWindow]::SetWindowPos($handle,[IntPtr]::Zero,0,0,0,0,0x43) | Out-Null
    [SealedWindow]::SetForegroundWindow($handle) | Out-Null
    if ([SealedWindow]::GetForegroundWindow() -ne $handle) {
        $thread=[SealedWindow]::GetCurrentThreadId()
        $foreground=[SealedWindow]::GetWindowThreadProcessId([SealedWindow]::GetForegroundWindow(),[IntPtr]::Zero)
        $attached=[SealedWindow]::AttachThreadInput($thread,$foreground,$true)
        try { [SealedWindow]::SetForegroundWindow($handle) | Out-Null }
        finally { if($attached) { [SealedWindow]::AttachThreadInput($thread,$foreground,$false) | Out-Null } }
    }
    if ([SealedWindow]::GetForegroundWindow() -ne $handle) { throw 'Target app cannot obtain foreground input; refusing unrelated input' }
}
function Post-AppKey([IntPtr]$handle, [int]$key, [int]$scan) {
    # Send explicit native window messages only to the app, without typing in a
    # user's foreground editor/browser. The winit adapter receives these keys.
    $extended=if ($key -ge 0x21 -and $key -le 0x28) {0x01000000} else {0}
    [SealedWindow]::PostMessage($handle,0x100,[IntPtr]$key,[IntPtr](1 -bor ($scan -shl 16) -bor $extended)) | Out-Null
    Start-Sleep -Milliseconds 100
    [SealedWindow]::PostMessage($handle,0x101,[IntPtr]$key,[IntPtr]([int64]0xC0000001L -bor ($scan -shl 16) -bor $extended)) | Out-Null
}
function Post-AppClick([IntPtr]$handle, [int]$x, [int]$y) {
    Prepare-AppWindow $handle -Foreground
    $screen=New-Object SealedWindow+Point
    $screen.X=$x; $screen.Y=$y
    if (-not [SealedWindow]::ClientToScreen($handle,[ref]$screen)) { throw 'Target client coordinates unavailable' }
    [SealedWindow]::SetCursorPos($screen.X,$screen.Y) | Out-Null
    Start-Sleep -Milliseconds 150
    $point=[IntPtr](($y -shl 16) -bor $x)
    [SealedWindow]::PostMessage($handle,0x200,[IntPtr]::Zero,$point) | Out-Null
    [SealedWindow]::PostMessage($handle,0x201,[IntPtr]1,$point) | Out-Null
    Start-Sleep -Milliseconds 100
    [SealedWindow]::PostMessage($handle,0x202,[IntPtr]::Zero,$point) | Out-Null
}
