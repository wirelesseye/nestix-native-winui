param(
    [Parameter(Mandatory = $true)]
    [string] $PackageRoot
)

$ErrorActionPreference = "Stop"

$packages = @(
    @{ Id = "Microsoft.WindowsAppSDK"; Version = "1.8.260529003"; Sha256 = "94390A8D7E8E5082178441F192F3341B82571A216D75722726ED20D9F42F20E2" },
    @{ Id = "Microsoft.WindowsAppSDK.Runtime"; Version = "1.8.260529003"; Sha256 = "255C763353E285B2DEF7ADE1323306346C052DB37473B3DFD90F822BA42BD44D" },
    @{ Id = "Microsoft.WindowsAppSDK.WinUI"; Version = "1.8.260528001"; Sha256 = "40630D73E19CEFA750DBE52D22D1E62BAE2F63687EB7A699749E620C48643544" },
    @{ Id = "Microsoft.WindowsAppSDK.Foundation"; Version = "1.8.260527000"; Sha256 = "32F642747CD39C0BF4F04EE53711915D7C8139426AC6DDC8E90979A3F11A7A16" },
    @{ Id = "Microsoft.WindowsAppSDK.Base"; Version = "1.8.251216001"; Sha256 = "58F0C69AD99293E7EFD36B7F8C6EAD0E20940A5E91866B16901B9610B08642C9" },
    @{ Id = "Microsoft.WindowsAppSDK.InteractiveExperiences"; Version = "1.8.260525001"; Sha256 = "C4D741C2188D6464B365C6FAD6C571720706667169ECF1229942A0877A2FB9CF" }
)

New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null

foreach ($package in $packages) {
    $id = $package.Id
    $version = $package.Version
    $lowerId = $id.ToLowerInvariant()
    $packageDir = Join-Path $PackageRoot $id
    $nupkgPath = Join-Path $packageDir "$id.$version.nupkg"
    $extractDir = Join-Path $packageDir $version
    $uri = "https://api.nuget.org/v3-flatcontainer/$lowerId/$version/$lowerId.$version.nupkg"

    New-Item -ItemType Directory -Force -Path $packageDir | Out-Null

    if (-not (Test-Path $nupkgPath)) {
        Write-Host "Downloading $id $version"
        Invoke-WebRequest -Uri $uri -OutFile $nupkgPath
    }

    $actualHash = (Get-FileHash -LiteralPath $nupkgPath -Algorithm SHA256).Hash
    if ($actualHash -ne $package.Sha256) {
        throw "SHA-256 mismatch for $nupkgPath"
    }

    if (-not (Test-Path $extractDir)) {
        $temporaryExtractDir = "$extractDir.extracting.$PID"
        New-Item -ItemType Directory -Force -Path $temporaryExtractDir | Out-Null
        Write-Host "Extracting $id $version"
        tar -xf $nupkgPath -C $temporaryExtractDir
        Move-Item -LiteralPath $temporaryExtractDir -Destination $extractDir
    }
}

Write-Host "Windows App SDK packages are ready under $PackageRoot"
