# Docker installed and its daemon answering - Windows.
# Saved with a BOM: Windows PowerShell reads a file without one as ANSI.
$ErrorActionPreference = 'SilentlyContinue'
function Answer($status, $summary, $detail) {
    $o = [ordered]@{ status = $status; summary = $summary }
    if ($detail) { $o.detail = "$detail" }
    $o | ConvertTo-Json -Compress
    exit 0
}
if (-not (Get-Command docker)) { Answer 'error' 'docker não está no PATH' }
$version = docker version --format '{{.Server.Version}}' 2>&1
if ($LASTEXITCODE -ne 0) { Answer 'error' 'O serviço do Docker não responde (o Docker Desktop está aberto?)' $version }
Answer 'ok' "Docker $version"
