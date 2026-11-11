# A fix that needs an administrator: it asks Windows for one (the UAC prompt)
# and runs itself again elevated, in a window of its own, since an elevated
# process cannot write to the tab that started it. The work here is harmless -
# a file in the temporary folder - and stands for what a real fix would do:
# a service, a firewall rule, a hosts entry.
param([switch]$Elevated)

$id = [Security.Principal.WindowsIdentity]::GetCurrent()
$admin = ([Security.Principal.WindowsPrincipal]$id).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Write-Host 'Esta correção precisa de administrador. O Windows vai perguntar...'
    try {
        $p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
            '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", '-Elevated')
        Write-Host "Concluído (código $($p.ExitCode)). Volte ao Diagnóstico e clique em Conferir."
    } catch {
        Write-Host 'A elevação foi recusada; nada foi feito.'
    }
    Read-Host 'Enter fecha esta aba'
    exit
}
$mark = Join-Path ([System.IO.Path]::GetTempPath()) 'consisterm-selftest-elevated.txt'
Set-Content $mark "elevado em $(Get-Date -Format s)"
Write-Host "Feito como administrador: $mark"
Start-Sleep 2
