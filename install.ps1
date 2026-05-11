# Rabbitty installer for Windows.
#   irm https://raw.githubusercontent.com/wHoIsDReAmer/RabbiTTY/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo = 'wHoIsDReAmer/RabbiTTY'
$InstallDir = Join-Path $env:LOCALAPPDATA 'Rabbitty'

function Get-LatestTag {
    $resp = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
    return $resp.tag_name
}

$tag = Get-LatestTag
if (-not $tag) {
    Write-Error 'failed to resolve latest release tag from GitHub.'
    exit 1
}

$asset = "rabbitty-$tag-windows-amd64.zip"
$url = "https://github.com/$Repo/releases/download/$tag/$asset"

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "rabbitty-install-$(Get-Random)") -Force
try {
    $zipPath = Join-Path $tmp.FullName $asset
    Write-Host "Downloading $asset..."
    Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing

    Write-Host 'Extracting...'
    Expand-Archive -Path $zipPath -DestinationPath $tmp.FullName -Force

    $exe = Get-ChildItem -Path $tmp.FullName -Recurse -Filter 'rabbitty.exe' | Select-Object -First 1
    if (-not $exe) {
        throw 'rabbitty.exe not found in archive.'
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -Path $exe.FullName -Destination (Join-Path $InstallDir 'rabbitty.exe') -Force

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($null -eq $userPath) -or ($userPath -notlike "*$InstallDir*")) {
        $newPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Host "Added $InstallDir to user PATH (restart terminal to pick it up)."
    }

    Write-Host ""
    Write-Host "Installed rabbitty.exe to $InstallDir"
    Write-Host "Run 'rabbitty' in a new terminal to start."
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
