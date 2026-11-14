# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Nidvue
#
# Build the Windows artifacts: an MSI when the WiX toolset is available, and a
# zip either way. WinFSP is a prerequisite installed separately.
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Version = (Select-String -Path "$Root\Cargo.toml" -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$Dist = Join-Path $Root "dist"
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

cargo build --release -p nidvue-cli -p nidvue-ffi --manifest-path "$Root\Cargo.toml"
if (Test-Path "$Root\apps\nidvue-ui\build\nidvue-ui.exe") {
    Copy-Item "$Root\apps\nidvue-ui\build\nidvue-ui.exe" $Dist -Force
}
Copy-Item "$Root\target\release\nidvue-cli.exe" $Dist -Force
Copy-Item "$Root\LICENSE", "$Root\NOTICE" $Dist -Force
@"
Corresponding source: https://github.com/Nidvue/nidvue-drive/tree/v$Version
WinFSP is a prerequisite: https://winfsp.dev (GPL-3.0 with FLOSS exception).
AGPL-3.0 sections 4-6 are satisfied by the tag above and the source archive.
"@ | Set-Content "$Dist\SOURCE-OFFER.txt"

$Zip = Join-Path $Dist "nidvue-drive-$Version-windows-x86_64.zip"
Compress-Archive -Path "$Dist\nidvue-cli.exe", "$Dist\nidvue-ui.exe", "$Dist\LICENSE", "$Dist\NOTICE", "$Dist\SOURCE-OFFER.txt" -DestinationPath $Zip -Force

if (Get-Command candle.exe -ErrorAction SilentlyContinue) {
    $Wxs = Join-Path $Dist "nidvue.wxs"
    @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*" Name="Nidvue Drive" Language="1033" Version="$Version"
           Manufacturer="Nidvue" UpgradeCode="7B5C1B7E-6F5B-4F0E-9F2A-1D9B0A1C2D3E">
    <Package InstallerVersion="500" Compressed="yes" InstallScope="perUser" />
    <MediaTemplate EmbedCab="yes" />
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLDIR" Name="Nidvue Drive">
          <Component Id="Cli" Guid="*">
            <File Id="CliExe" Source="nidvue-cli.exe" KeyPath="yes" />
          </Component>
          <Component Id="Ui" Guid="*">
            <File Id="UiExe" Source="nidvue-ui.exe" KeyPath="yes" />
          </Component>
        </Directory>
      </Directory>
    </Directory>
    <Feature Id="Main" Level="1">
      <ComponentRef Id="Cli" />
      <ComponentRef Id="Ui" />
    </Feature>
  </Product>
</Wix>
"@ | Set-Content $Wxs
    candle.exe -out "$Dist\nidvue.wixobj" $Wxs
    light.exe -out "$Dist\nidvue-drive-$Version-windows-x86_64.msi" "$Dist\nidvue.wixobj"
} else {
    Write-Host "WiX not installed: shipping the zip only"
}

Get-FileHash "$Dist\*.zip", "$Dist\*.msi" -ErrorAction SilentlyContinue |
    ForEach-Object { "$($_.Hash.ToLower())  $($_.Path | Split-Path -Leaf)" } |
    Set-Content "$Dist\SHA256SUMS"
Write-Host "windows artifacts in $Dist"
