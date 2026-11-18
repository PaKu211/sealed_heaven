# C05 additive compatibility check; never rewrite the accepted C04 evidence log.
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$cargo=Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
$git='C:/Program Files/Git/cmd/git.exe'
$python='C:/Python314/python.exe'
$base='f35bfd3664d4baf2feaf78755b1a18d29cccf795'
$log=Join-Path $root 'C0_5_VERIFICATION_LOG.txt'
$utf8=New-Object Text.UTF8Encoding($false)
[IO.File]::WriteAllText($log,"C0.5 verification $(Get-Date -Format o)`n",$utf8)
function Run([string]$exe,[string[]]$parameters) {
    [IO.File]::AppendAllText($log,"`n> $exe $($parameters -join ' ')`n",$utf8)
    $ErrorActionPreference='Continue'
    $output=& $exe @parameters 2>&1 | ForEach-Object {$_.ToString()}
    $code=$LASTEXITCODE
    $ErrorActionPreference='Stop'
    $text=$output|Out-String
    [IO.File]::AppendAllText($log,$text+"exit=$code`n",$utf8)
    Write-Output $text
    if($code -ne 0) {throw "Failed: $exe $($parameters -join ' ')"}
}
Push-Location $root
try {
    Run $git @('rev-parse','HEAD')
    Run $git @('merge-base','--is-ancestor',$base,'HEAD')
    # The sole modification of an existing core source is the new module export.
    $old=(& $git show "${base}:src/lib.rs") -join "`n"
    $new=([IO.File]::ReadAllText((Join-Path $root 'src/lib.rs')) -replace "`r`n","`n").TrimEnd("`n")
    if(($new -replace '(?m)^pub mod player5;\n','') -cne $old) {throw 'Existing src/lib.rs changed beyond C05 module export'}
    $paths=@(& $git ls-tree -r --name-only $base -- src Cargo.toml Cargo.lock rust-toolchain.toml docs/C0_1_IMPLEMENTATION_SPEC.md docs/C0_1_ACCEPTANCE_TESTS.md docs/C0_2_IMPLEMENTATION_SPEC.md docs/C0_2_ACCEPTANCE_TESTS.md docs/C0_3_IMPLEMENTATION_SPEC.md docs/C0_3_ACCEPTANCE_TESTS.md docs/C0_4_IMPLEMENTATION_SPEC.md docs/C0_4_ACCEPTANCE_TESTS.md docs/tools apps/sealed_playable/assets apps/sealed_playable/locales/en.json apps/sealed_playable/locales/id.json apps/sealed_playable/src/desktop.rs apps/sealed_playable/src/view.rs apps/sealed_playable/src/persistence.rs apps/sealed_playable/tests/adapter.rs)
    $paths=@($paths|Where-Object {$_ -ne 'src/lib.rs'})
    Run $git (@('diff','--exit-code',$base,'--')+$paths)
    [IO.File]::AppendAllText($log,"Frozen implementations/contracts/vectors/assets/dependencies: unchanged; src/lib.rs additive export only`n",$utf8)
    Run $cargo @('--version')
    Run $cargo @('fmt','--all','--check')
    Run $cargo @('check','--all-targets','--locked','--offline')
    Run $cargo @('clippy','--all-targets','--locked','--offline','--','-D','warnings')
    Run $cargo @('test','--all-targets','--locked','--offline')
    Run $cargo @('test','--doc','--locked','--offline')
    Run $cargo @('test','--all-targets','--release','--locked','--offline')
    Run $cargo @('test','--doc','--release','--locked','--offline')
    Run $python @('docs/tools/c03_vectors.py','--check')
    Run $python @('apps/sealed_playable/tools/pack_demo.py','--check')
    $manifest=@('--manifest-path','apps/sealed_playable/Cargo.toml')
    Run $cargo (@('fmt')+$manifest+@('--all','--check'))
    Run $cargo (@('check')+$manifest+@('--all-targets','--locked','--offline'))
    Run $cargo (@('clippy')+$manifest+@('--all-targets','--locked','--offline','--','-D','warnings'))
    Run $cargo (@('test')+$manifest+@('--locked','--offline'))
    Run $cargo (@('test')+$manifest+@('--release','--locked','--offline'))
    Run $cargo (@('build')+$manifest+@('--bins','--locked','--offline'))
    Run $cargo (@('build')+$manifest+@('--bins','--release','--locked','--offline'))
    [IO.File]::AppendAllText($log,"`nAutomated verification complete; native gameplay/performance and independent acceptance are separate gates.`n",$utf8)
} finally {Pop-Location}
