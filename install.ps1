$ErrorActionPreference = "Stop"

$Repo = if ($env:XSEARCH_REPO) { $env:XSEARCH_REPO } else { "catoncat/xsearch" }
$Dest = if ($env:XSEARCH_INSTALL_DIR) { $env:XSEARCH_INSTALL_DIR } else { Join-Path $HOME ".agents\skills\xsearch" }
$ConfigRoot = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else { Join-Path $HOME ".config" }
$ConfigDir = Join-Path $ConfigRoot "xsearch"

if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    throw "Unsupported Windows architecture: $env:PROCESSOR_ARCHITECTURE. Build from source instead."
}

$Version = if ($env:XSEARCH_VERSION) {
    $env:XSEARCH_VERSION
} else {
    (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name
}

$Target = "x86_64-pc-windows-msvc"
$Asset = "xsearch-$Target.zip"
$Base = "https://github.com/$Repo/releases/download/$Version"
$Raw = "https://raw.githubusercontent.com/$Repo/$Version"
$Temp = Join-Path ([System.IO.Path]::GetTempPath()) ("xsearch-" + [guid]::NewGuid())

try {
    New-Item -ItemType Directory -Force -Path $Temp | Out-Null
    Write-Host "Installing xsearch $Version for $Target..."
    Invoke-WebRequest "$Base/$Asset" -OutFile (Join-Path $Temp $Asset)
    Invoke-WebRequest "$Base/checksums.txt" -OutFile (Join-Path $Temp "checksums.txt")

    $ChecksumLine = Get-Content (Join-Path $Temp "checksums.txt") | Where-Object { $_ -match "\s$([regex]::Escape($Asset))$" }
    if (-not $ChecksumLine) { throw "Checksum missing for $Asset" }
    $Expected = ($ChecksumLine -split "\s+")[0].ToLower()
    $Actual = (Get-FileHash (Join-Path $Temp $Asset) -Algorithm SHA256).Hash.ToLower()
    if ($Expected -ne $Actual) { throw "Checksum verification failed" }

    Expand-Archive (Join-Path $Temp $Asset) -DestinationPath $Temp -Force
    New-Item -ItemType Directory -Force -Path (Join-Path $Dest "bin"), $ConfigDir | Out-Null
    Copy-Item (Join-Path $Temp "xsearch.exe") (Join-Path $Dest "bin\xsearch.exe") -Force
    Invoke-WebRequest "$Raw/SKILL.md" -OutFile (Join-Path $Dest "SKILL.md")
    Invoke-WebRequest "$Raw/config.example.toml" -OutFile (Join-Path $Dest "config.example.toml")

    $ConfigFile = Join-Path $ConfigDir "config.toml"
    if (-not (Test-Path $ConfigFile)) {
        Copy-Item (Join-Path $Dest "config.example.toml") $ConfigFile
        Write-Host "Created $ConfigFile; add your proxy endpoint and model."
    }

    Write-Host "Installed binary: $(Join-Path $Dest 'bin\xsearch.exe')"
    Write-Host "Installed skill:  $(Join-Path $Dest 'SKILL.md')"
    & (Join-Path $Dest "bin\xsearch.exe") --version
} finally {
    Remove-Item $Temp -Recurse -Force -ErrorAction SilentlyContinue
}
