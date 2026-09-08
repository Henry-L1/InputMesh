param(
    [ValidateSet("Key", "MouseMove", "MouseButton")]
    [string]$Mode,
    [int]$DeltaX = 0,
    [int]$DeltaY = 0,
    [ValidateSet("Down", "Up")]
    [string]$ButtonState = "Down",
    [string]$ResultPath = ""
)

$ErrorActionPreference = "Stop"
$repository = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$result = if ($ResultPath) { $ResultPath } else { Join-Path $repository "windows-physical-input-result.json" }

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class InputMeshPhysicalInputProbe {
    [StructLayout(LayoutKind.Sequential)]
    public struct Point { public int X; public int Y; }

    [DllImport("user32.dll")]
    public static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    public static extern bool GetCursorPos(out Point point);
}
"@

$before = New-Object InputMeshPhysicalInputProbe+Point
$after = New-Object InputMeshPhysicalInputProbe+Point
[void][InputMeshPhysicalInputProbe]::GetCursorPos([ref]$before)

if ($Mode -eq "Key") {
    $virtualKeyF9 = 0x78
    [InputMeshPhysicalInputProbe]::keybd_event($virtualKeyF9, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 50
    [InputMeshPhysicalInputProbe]::keybd_event($virtualKeyF9, 0, 2, [UIntPtr]::Zero)
}
elseif ($Mode -eq "MouseMove") {
    [InputMeshPhysicalInputProbe]::mouse_event(1, $DeltaX, $DeltaY, 0, [UIntPtr]::Zero)
}
else {
    $flag = if ($ButtonState -eq "Down") { 2 } else { 4 }
    [InputMeshPhysicalInputProbe]::mouse_event($flag, 0, 0, 0, [UIntPtr]::Zero)
}

Start-Sleep -Milliseconds 150
[void][InputMeshPhysicalInputProbe]::GetCursorPos([ref]$after)
[ordered]@{
    ok = $true
    mode = $Mode
    buttonState = $ButtonState
    delta = @{ x = $DeltaX; y = $DeltaY }
    before = @{ x = $before.X; y = $before.Y }
    after = @{ x = $after.X; y = $after.Y }
    extraInfo = 0
} | ConvertTo-Json -Compress | Set-Content -LiteralPath $result -Encoding utf8
