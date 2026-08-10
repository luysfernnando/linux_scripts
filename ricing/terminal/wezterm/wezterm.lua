local wezterm = require 'wezterm'
local config = wezterm.config_builder()

-- Shell padrão: PowerShell
config.default_prog = { 'pwsh.exe', '-NoLogo' }

-- Windows engole Alt pra compose de tecla morta por padrão; sem isso Alt+V
-- (usado pelo Claude Code CLI pra colar imagem) nunca chega no app como Meta+V.
config.send_composed_key_when_left_alt_is_pressed = false

-- Aparência geral
config.color_scheme = 'Catppuccin Mocha' -- outras opções: 'Tokyo Night', 'Dracula', 'Nightfox'
config.window_background_opacity = 0.90
config.text_background_opacity = 1.0

-- Fonte (precisa ter instalada - baixe uma Nerd Font)
config.font = wezterm.font_with_fallback({
  'JetBrainsMono Nerd Font',
  'Maple Mono NF',
})
config.font_size = 11.0
config.line_height = 1.1

-- Tab bar estilizada (fake title bar, parece com o da imagem)
config.enable_tab_bar = true
config.use_fancy_tab_bar = false
config.tab_bar_at_bottom = true
config.hide_tab_bar_if_only_one_tab = false
config.tab_max_width = 24

-- Bordas / janela
config.window_decorations = "RESIZE" -- tira a barra de título nativa do Windows
config.window_padding = {
  left = 8,
  right = 8,
  top = 8,
  bottom = 0,
}

-- Cores da tab bar iguais ao fundo do terminal (integrada, sem faixa separada)
config.colors = {
  tab_bar = {
    background = '#1e1e2e',
    active_tab = {
      bg_color = '#1e1e2e',
      fg_color = '#cdd6f4',
    },
    inactive_tab = {
      bg_color = '#1e1e2e',
      fg_color = '#6c7086',
    },
    inactive_tab_hover = {
      bg_color = '#313244',
      fg_color = '#cdd6f4',
    },
    new_tab = {
      bg_color = '#1e1e2e',
      fg_color = '#6c7086',
    },
  },
}

-- Cursor
config.default_cursor_style = 'SteadyBar'
config.cursor_blink_rate = 500

-- Habilita renderização de imagem no fastfetch (protocolo Kitty)
config.enable_kitty_graphics = true

-- Scrollback
config.scrollback_lines = 5000

-- Performance: GPU acceleration
config.front_end = 'WebGpu'
config.webgpu_power_preference = 'HighPerformance'
config.max_fps = 120
config.animation_fps = 60

-- Keybinds
config.keys = {
  {
    key = 'q',
    mods = 'CTRL',
    action = wezterm.action.CloseCurrentTab { confirm = false },
  },
  {
    key = 't',
    mods = 'CTRL',
    action = wezterm.action.SpawnTab 'CurrentPaneDomain',
  },
  {
    key = 'c',
    mods = 'CTRL',
    action = wezterm.action.CopyTo 'Clipboard',
  },
  {
    key = 'c',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.SendString '\x03',
  },
  {
    -- Ctrl+V: clipboard com imagem -> salva PNG em temp e cola o caminho.
    -- Alt+V (feature nativa do Claude Code CLI pra imagem) não funcionou
    -- aqui — ficou só nesse fallback via path mesmo.
    -- Sem imagem -> cola texto normal.
    key = 'v',
    mods = 'CTRL',
    action = wezterm.action_callback(function(window, pane)
      local script = [[
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Get-ChildItem "$env:TEMP\wezterm-paste-*.png" -ErrorAction SilentlyContinue |
  Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-1) } |
  Remove-Item -Force -ErrorAction SilentlyContinue
if ([System.Windows.Forms.Clipboard]::ContainsImage()) {
  $img = [System.Windows.Forms.Clipboard]::GetImage()
  $path = Join-Path $env:TEMP ("wezterm-paste-" + [guid]::NewGuid().ToString("N") + ".png")
  $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  Write-Output $path
}
]]
      local ok, stdout = wezterm.run_child_process {
        'powershell.exe', '-NoProfile', '-Sta', '-Command', script,
      }
      local path = ok and stdout and stdout:gsub('%s+$', '') or ''
      if path ~= '' then
        pane:send_text('"' .. path .. '"')
      else
        window:perform_action(wezterm.action.PasteFrom 'Clipboard', pane)
      end
    end),
  },
}

return config
