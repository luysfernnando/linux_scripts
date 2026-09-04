Import-Module Terminal-Icons
Invoke-Expression (&starship init powershell)

# `fastfetch --full` mostra tudo (host, board, bios, disco, som, IP...); sem
# argumento mostra o perfil enxuto. `--full` não é flag do fastfetch, é atalho
# pro segundo config gerado pelo install-menu.sh.
function fastfetch {
    $exe = (Get-Command fastfetch.exe -ErrorAction SilentlyContinue).Source
    if (-not $exe) { Write-Error "fastfetch.exe não encontrado"; return }
    if ($args -contains '--full') {
        & $exe --config full @($args | Where-Object { $_ -ne '--full' })
    } else {
        & $exe @args
    }
}
