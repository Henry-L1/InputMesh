param(
    [string]$ProbeName = "inputmesh_windows_probe",
    [string]$ResultName = "windows-input-probe-result.json"
)

$ErrorActionPreference = "Stop"

$repository = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$probe = Join-Path $repository "src-tauri\vendor\rdev\target\debug\examples\$ProbeName.exe"
$result = Join-Path $repository $ResultName

try {
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $probe 2>&1
    $nativeExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorAction
    if ($nativeExitCode -ne 0) {
        $renderedOutput = ($output | Out-String).Trim()
        throw "input probe exited with code $nativeExitCode`: $renderedOutput"
    }
    Set-Content -LiteralPath $result -Value $output -Encoding utf8
}
catch {
    $details = ($_ | Out-String).Trim()
    if ([string]::IsNullOrWhiteSpace($details)) {
        $details = "unknown probe failure"
    }
    $failure = [ordered]@{
        ok = $false
        error = $details
        exitCode = $LASTEXITCODE
        probe = $probe
    } | ConvertTo-Json -Compress
    Set-Content -LiteralPath $result -Value $failure -Encoding utf8
    exit 1
}
