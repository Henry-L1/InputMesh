param(
    [string]$ResultPath = ""
)

$ErrorActionPreference = "Stop"

$repository = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$result = if ($ResultPath) { $ResultPath } else { Join-Path $repository "windows-key-observer-result.json" }

Add-Type @"
using System.Runtime.InteropServices;

public static class InputMeshKeyboardState {
    [DllImport("user32.dll")]
    public static extern short GetAsyncKeyState(int virtualKey);
}
"@

$virtualKey = 0x78
$keyName = "F9"
$sawDown = $false
$deadline = [DateTime]::UtcNow.AddSeconds(30)
while ([DateTime]::UtcNow -lt $deadline) {
    $isDown = ([InputMeshKeyboardState]::GetAsyncKeyState($virtualKey) -band 0x8000) -ne 0
    if ($isDown) {
        $sawDown = $true
    }
    elseif ($sawDown) {
        [pscustomobject]@{
            ok = $true
            key = $keyName
            states = @("down", "up")
            source = "GetAsyncKeyState"
        } | ConvertTo-Json -Compress | Set-Content -LiteralPath $result -Encoding utf8
        exit 0
    }
    Start-Sleep -Milliseconds 10
}

$failure = [pscustomobject]@{
    ok = $false
    error = "did not observe an F9 press/release pair"
}
$failure | ConvertTo-Json -Compress | Set-Content -LiteralPath $result -Encoding utf8
exit 1
