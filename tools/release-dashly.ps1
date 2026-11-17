# Builds a Euro-Hub release for the fork (Luc-nbr/WestPlay): the game plugin, probe and map builder, then the signed
# installer and the updater's latest.json, which Euro-Hub's installs read from the fork's latest release.
#
#   powershell -ExecutionPolicy Bypass -File tools\release-dashly.ps1 -Version 2.5.0 -Notes "What changed"
#   ... then, to publish it on GitHub (your own step):
#   powershell -ExecutionPolicy Bypass -File tools\release-dashly.ps1 -Version 2.5.0 -Publish
#
# The signing key stays on this PC (C:\Users\<you>\.tauri\dashly.key); its public half is in app\tauri.conf.json.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Notes = "",
    [switch]$Publish
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$repo = 'Luc-nbr/WestPlay'
$out = Join-Path $root "target\release\bundle\nsis"
$setup = "Euro-Hub_${Version}_x64-setup.exe"

if ($Publish) {
    $files = @((Join-Path $out $setup), (Join-Path $out 'latest.json'))
    foreach ($file in $files) { if (-not (Test-Path $file)) { throw "Build $Version first: $file is missing" } }
    gh release create "v$Version" @files --repo $repo --title "Euro-Hub $Version" --notes $Notes
    exit $LASTEXITCODE
}

# The version, everywhere it's kept (UTF-8 without a BOM: Windows PowerShell's utf8 writes one, which breaks them).
function Set-Text([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding $false))
}
$cargo = Join-Path $root 'Cargo.toml'
Set-Text $cargo ((Get-Content $cargo -Raw -Encoding utf8) -replace '(?m)^version = "[^"]+"', "version = `"$Version`"")
$conf = Join-Path $root 'app\tauri.conf.json'
Set-Text $conf ((Get-Content $conf -Raw -Encoding utf8) -replace '"version": "[^"]+"', "`"version`": `"$Version`"")
$package = Join-Path $root 'ui\package.json'
Set-Text $package ((Get-Content $package -Raw -Encoding utf8) -replace '"version": "[^"]+"', "`"version`": `"$Version`"")

Push-Location $root
try {
    cargo build --release -p westplay-plugin -p westplay-probe -p westplay-maps
    if ($LASTEXITCODE) { throw 'the plugin, probe or map builder did not build' }

    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content (Join-Path $env:USERPROFILE '.tauri\dashly.key') -Raw
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
    Push-Location app
    npx tauri build --config tauri.release.conf.json
    $built = $LASTEXITCODE
    Pop-Location
    if ($built) { throw 'the installer did not build' }

    $signature = Get-Content (Join-Path $out "$setup.sig") -Raw
    $latest = [ordered]@{
        version   = $Version
        notes     = $Notes
        pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
        platforms = [ordered]@{
            'windows-x86_64' = [ordered]@{
                signature = $signature.Trim()
                url       = "https://github.com/$repo/releases/download/v$Version/$setup"
            }
        }
    }
    Set-Text (Join-Path $out 'latest.json') ($latest | ConvertTo-Json -Depth 5)
    Write-Host "Built: $(Join-Path $out $setup) and latest.json. Publish with -Publish when you're ready."
}
finally {
    Pop-Location
}
