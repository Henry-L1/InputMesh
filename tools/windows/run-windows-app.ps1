$ErrorActionPreference = "Stop"

$repository = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$executable = Join-Path $repository "src-tauri\target\release\inputmesh.exe"
$stdout = Join-Path $repository "inputmesh-windows.stdout.log"
$stderr = Join-Path $repository "inputmesh-windows.stderr.log"
$pidFile = Join-Path $repository "inputmesh-windows.pid"

$env:RUST_LOG = "inputmesh=debug,warn"
Remove-Item -LiteralPath $stdout, $stderr, $pidFile -Force -ErrorAction SilentlyContinue
$process = Start-Process -FilePath $executable `
    -WorkingDirectory $repository `
    -RedirectStandardOutput $stdout `
    -RedirectStandardError $stderr `
    -PassThru
Set-Content -LiteralPath $pidFile -Value $process.Id -Encoding ascii
