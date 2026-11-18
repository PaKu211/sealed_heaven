# Repeatable Windows verification. Root and application locks stay separate.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$cargo = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
$git = 'C:/Program Files/Git/cmd/git.exe'
$python = 'C:/Python314/python.exe'
$log = Join-Path $root 'C0_4_VERIFICATION_LOG.txt'
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText($log, "C0.4 verification $(Get-Date -Format o)`n", $utf8)
function Run-Check([string]$exe, [string[]]$parameters) {
    [IO.File]::AppendAllText($log, "`n> $exe $($parameters -join ' ')`n", $utf8)
    $ErrorActionPreference = 'Continue' # native progress is often written to stderr
    $output = & $exe @parameters 2>&1 | ForEach-Object { $_.ToString() }
    $code = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $text = ($output | Out-String)
    [IO.File]::AppendAllText($log, $text, $utf8)
    Write-Output $text
    [IO.File]::AppendAllText($log, "exit=$code`n", $utf8)
    if ($code -ne 0) { throw "Verification failed: $exe $($parameters -join ' ')" }
}
Push-Location $root
try {
    Run-Check $git @('rev-parse', 'HEAD')
    $frozen = & $git diff --name-only 14d229bedc308d7e50225122caaec0ce5732a201 -- src Cargo.toml Cargo.lock rust-toolchain.toml 'docs/C0_1*' 'docs/C0_2*' 'docs/C0_3*' docs/tools/c03_vectors.py
    if ($LASTEXITCODE -ne 0 -or $frozen) { throw "Frozen files changed: $frozen" }
    [IO.File]::AppendAllText($log, "Frozen baseline diff: empty`n", $utf8)
    Run-Check $cargo @('--version')
    Run-Check $cargo @('fmt', '--all', '--check')
    Run-Check $cargo @('check', '--locked', '--offline')
    Run-Check $cargo @('clippy', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings')
    Run-Check $cargo @('test', '--locked', '--offline')
    Run-Check $cargo @('test', '--release', '--locked', '--offline')
    Run-Check $python @('docs/tools/c03_vectors.py', '--check')
    Run-Check $python @('apps/sealed_playable/tools/pack_demo.py', '--check')
    $manifest = @('--manifest-path', 'apps/sealed_playable/Cargo.toml')
    Run-Check $cargo (@('fmt') + $manifest + @('--all', '--check'))
    Run-Check $cargo (@('check') + $manifest + @('--locked', '--offline'))
    Run-Check $cargo (@('clippy') + $manifest + @('--all-targets', '--locked', '--offline', '--', '-D', 'warnings'))
    Run-Check $cargo (@('test') + $manifest + @('--locked', '--offline'))
    Run-Check $cargo (@('test') + $manifest + @('--release', '--locked', '--offline'))
    Run-Check $cargo (@('build') + $manifest + @('--locked', '--offline'))
    [IO.File]::AppendAllText($log, "`nAutomated verification complete. Native interactive smoke and independent final review are separate gates.`n", $utf8)
} finally { Pop-Location }
