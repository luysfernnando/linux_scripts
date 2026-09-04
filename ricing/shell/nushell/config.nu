# config.nu — Nushell no Windows. Symlinkado pra
# %APPDATA%\nushell\config.nu pelo install-menu.sh.

use ~/.cache/starship/init.nu

# Linha em branco antes do prompt, menos depois do fastfetch.
#
# O `add_newline` do starship faria isso, mas ele não sabe distinguir: a mesma
# linha que dá respiro entre comandos vira vão embaixo do logo do fastfetch, que
# já termina colado no prompt. Então fica desligado no windows.toml e quem
# imprime é este hook, que o fastfetch consegue suprimir por uma execução.
#
# Hook e não closure no PROMPT_COMMAND porque só bloco de hook consegue escrever
# em $env — closure roda num escopo que descarta a mudança.
$env.FF_SKIP_BLANK = false
$env.config.hooks.pre_prompt = ($env.config.hooks.pre_prompt | append {||
  if $env.FF_SKIP_BLANK { $env.FF_SKIP_BLANK = false } else { print "" }
})

# `fastfetch --full` mostra tudo (host, board, bios, disco, som, IP...); sem
# argumento mostra o perfil enxuto. `--full` não é flag do fastfetch, é atalho
# pro segundo config gerado pelo install-menu.sh.
#
# `--env` pra que o FF_SKIP_BLANK sobreviva ao fim do comando.
def --env --wrapped fastfetch [...rest] {
  if "--full" in $rest {
    ^fastfetch --config full ...($rest | where {|a| $a != "--full"})
  } else {
    ^fastfetch ...$rest
  }
  $env.FF_SKIP_BLANK = true
}
