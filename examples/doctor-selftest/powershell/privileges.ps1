# Self-test, PowerShell: whether the check runs elevated. The Doctor never
# elevates a check; a fix that needs it asks - see elevated_fix.ps1.
# -Require turns "not elevated" into a warning, so that fix is offered.
param([switch]$Require)
$admin = $false
if ($IsWindows -or $env:OS -eq 'Windows_NT') {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $admin = ([Security.Principal.WindowsPrincipal]$id).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
} else {
    $admin = ((id -u) -eq '0')
}
[ordered]@{
    status  = $(if ($Require -and -not $admin) { 'warn' } else { 'ok' })
    summary = $(if ($admin) { 'Rodando como administrador' } else { 'Rodando sem elevação (o normal)' })
} | ConvertTo-Json -Compress
