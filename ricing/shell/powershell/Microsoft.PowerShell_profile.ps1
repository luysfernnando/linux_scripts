Import-Module Terminal-Icons
Invoke-Expression (&starship init powershell)

# Linha em branco antes do prompt, menos depois do fastfetch.
#
# O `add_newline` do starship faria isso, mas ele não sabe distinguir: a mesma
# linha que dá respiro entre comandos vira vão embaixo do logo do fastfetch, que
# já termina colado no prompt. Então fica desligado no windows.toml e quem
# imprime é este wrapper, que o fastfetch consegue suprimir por uma execução.
$global:FfSkipBlankLine = $false
$global:StarshipPrompt = (Get-Item Function:prompt).ScriptBlock

function prompt {
    # O starship roda PRIMEIRO, com $? e $LASTEXITCODE do comando anterior
    # intactos — ele usa os dois pra pintar o ❯ de vermelho quando algo falha.
    # Por isso a linha em branco entra no texto do prompt em vez de sair num
    # Write-Host antes: qualquer comando nosso aqui sobrescreveria o $?.
    $rendered = & $global:StarshipPrompt
    if ($global:FfSkipBlankLine) {
        $global:FfSkipBlankLine = $false
        $rendered
    } else {
        "`n" + $rendered
    }
}

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
    $global:FfSkipBlankLine = $true
}
