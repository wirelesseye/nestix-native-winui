param(
    [string] $PackageRoot = ".packages",
    [switch] $Force
)

$ErrorActionPreference = "Stop"

$packages = @(
    @{ Id = "Microsoft.WindowsAppSDK"; Version = "1.8.260529003" },
    @{ Id = "Microsoft.WindowsAppSDK.WinUI"; Version = "1.8.260528001" },
    @{ Id = "Microsoft.WindowsAppSDK.Foundation"; Version = "1.8.260527000" },
    @{ Id = "Microsoft.WindowsAppSDK.Base"; Version = "1.8.251216001" },
    @{ Id = "Microsoft.WindowsAppSDK.InteractiveExperiences"; Version = "1.8.260525001" }
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

    if ($Force -or -not (Test-Path $nupkgPath)) {
        Write-Host "Downloading $id $version"
        Invoke-WebRequest -Uri $uri -OutFile $nupkgPath
    } else {
        Write-Host "Using cached $nupkgPath"
    }

    if ($Force -and (Test-Path $extractDir)) {
        Remove-Item -LiteralPath $extractDir -Recurse -Force
    }

    if (-not (Test-Path $extractDir)) {
        New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
        Write-Host "Extracting $id $version"
        tar -xf $nupkgPath -C $extractDir
    } else {
        Write-Host "Using extracted $extractDir"
    }
}

Write-Host "Windows App SDK packages are ready under $PackageRoot"
