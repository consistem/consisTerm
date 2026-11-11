# Self-test, PowerShell: a git update in a temporary folder - a repository
# gets a commit, and a clone of it is brought up to date with `git pull`.
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    '{"status":"skip","summary":"git não está instalado"}'; exit 0
}
function G { & git -c user.name=Doctor -c user.email=doctor@example.invalid -c init.defaultBranch=main @args 2>&1 | Out-Null; $LASTEXITCODE -eq 0 }

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("consisterm-selftest-git-" + [guid]::NewGuid())
New-Item -ItemType Directory $root | Out-Null
try {
    $origin = Join-Path $root 'origin.git'; $work = Join-Path $root 'work'; $mine = Join-Path $root 'mine'
    $ready = (G init --bare $origin) -and (G clone $origin $work)
    Set-Content (Join-Path $work 'versao.txt') '8.0.46b2'
    $ready = $ready -and (G -C $work add .) -and (G -C $work commit -m primeira) -and (G -C $work push origin HEAD:main) -and (G clone $origin $mine)
    if (-not $ready) { '{"status":"error","summary":"não foi possível preparar o repositório"}'; exit 0 }

    Set-Content (Join-Path $work 'versao.txt') '8.0.46b3'
    $null = (G -C $work commit -am 'nova versão') -and (G -C $work push origin HEAD:main)
    $before = git -C $mine rev-parse HEAD
    $null = G -C $mine pull --ff-only
    $after = git -C $mine rev-parse HEAD
    $version = (Get-Content (Join-Path $mine 'versao.txt')).Trim()
    if ($before -ne $after -and $version -eq '8.0.46b3') { '{"status":"ok","summary":"git pull atualizou o clone"}' }
    else { '{"status":"error","summary":"git pull não atualizou"}' }
}
finally { Remove-Item $root -Recurse -Force -ErrorAction SilentlyContinue }
