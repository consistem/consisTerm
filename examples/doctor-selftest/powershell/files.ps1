# Self-test, PowerShell: folders and files - create, write, copy, move, edit
# one line, rename, remove. Saved with a BOM: Windows PowerShell reads a file
# without one as ANSI.
$ErrorActionPreference = 'Stop'
$steps = [System.Collections.Generic.List[string]]::new()
$failed = 0
function Step($name, [bool]$ok) {
    if ($ok) { $steps.Add("ok: $name") } else { $steps.Add("FALHOU: $name"); $script:failed++ }
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("consisterm-selftest-ps-" + [guid]::NewGuid())
try {
    $deep = New-Item -ItemType Directory -Path (Join-Path $root 'a\b\c') -Force
    Step 'criar pastas aninhadas' (Test-Path $deep)

    $env = Join-Path $deep 'settings.env'
    [System.IO.File]::WriteAllText($env, "APP_NAME=example`nAPP_VERSION=8.0.46b2`nTZ=America/Sao_Paulo`n")
    Step 'gravar arquivo' ((Get-Item $env).Length -gt 0)

    Copy-Item $env (Join-Path $root 'settings.env.bak')
    Step 'copiar arquivo' (Test-Path (Join-Path $root 'settings.env.bak'))

    $moved = New-Item -ItemType Directory -Path (Join-Path $root 'moved') -Force
    Move-Item $env $moved
    $env = Join-Path $moved 'settings.env'
    Step 'mover arquivo' ((Test-Path $env) -and -not (Test-Path (Join-Path $deep 'settings.env')))

    $text = [System.IO.File]::ReadAllText($env) -replace '(?m)^APP_VERSION=.*$', 'APP_VERSION=8.0.46b3'
    [System.IO.File]::WriteAllText($env, $text)
    $after = [System.IO.File]::ReadAllText($env)
    Step 'editar uma linha específica' (($after -match '(?m)^APP_VERSION=8\.0\.46b3$') -and ($after -match 'TZ=America/Sao_Paulo'))

    Rename-Item $moved 'renamed'
    Step 'renomear pasta' (Test-Path (Join-Path $root 'renamed'))
}
catch { Step "exceção: $($_.Exception.Message)" $false }
finally { Remove-Item $root -Recurse -Force -ErrorAction SilentlyContinue }
Step 'apagar a árvore' (-not (Test-Path $root))

$total = $steps.Count
[ordered]@{
    status  = $(if ($failed) { 'error' } else { 'ok' })
    summary = "$($total - $failed) de $total operações de arquivo"
    detail  = @($steps)
} | ConvertTo-Json -Compress
