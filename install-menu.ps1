#!/usr/bin/env pwsh
# Wrapper pra rodar install-menu.sh (bash) a partir do PowerShell/duplo-clique.
$ErrorActionPreference = "Stop"

$bashCmd = Get-Command bash -ErrorAction SilentlyContinue
$bashPath = if ($bashCmd) { $bashCmd.Source } else { $null }

if (-not $bashPath) {
    $candidates = @(
        "$env:ProgramFiles\Git\bin\bash.exe",
        "${env:ProgramFiles(x86)}\Git\bin\bash.exe",
        "$env:LOCALAPPDATA\Programs\Git\bin\bash.exe"
    )
    $bashPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}

if (-not $bashPath) {
    Write-Error "faltando: bash (Git Bash). Instale com: winget install --id Git.Git -e"
    exit 1
}

$repoDir = Split-Path -Parent $MyInvocation.MyCommand.Path
& $bashPath "$repoDir/install-menu.sh"
exit $LASTEXITCODE
