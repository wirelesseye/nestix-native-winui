param(
    [string] $PackageRoot = ".packages",
    [string] $Output = "nestix-native-winui-build/app.manifest"
)

$ErrorActionPreference = "Stop"

$fragments = @(
    (Join-Path $PackageRoot "Microsoft.WindowsAppSDK.Foundation/1.8.260527000/runtimes-framework/package.appxfragment"),
    (Join-Path $PackageRoot "Microsoft.WindowsAppSDK.WinUI/1.8.260528001/runtimes-framework/package.appxfragment"),
    (Join-Path $PackageRoot "Microsoft.WindowsAppSDK.InteractiveExperiences/1.8.260525001/runtimes-framework/package.appxfragment")
)

$lines = [Collections.Generic.List[string]]::new()
$lines.Add('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>')
$lines.Add('<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3" xmlns:winrtv1="urn:schemas-microsoft-com:winrt.v1">')
$lines.Add('  <assemblyIdentity version="1.0.0.0" processorArchitecture="*" name="Nestix.Native.WinUI.Application" type="win32"/>')
$lines.Add('  <description>Nestix native WinUI application</description>')
$lines.Add('  <dependency>')
$lines.Add('    <dependentAssembly>')
$lines.Add('      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>')
$lines.Add('    </dependentAssembly>')
$lines.Add('  </dependency>')
$lines.Add('  <application xmlns="urn:schemas-microsoft-com:asm.v3">')
$lines.Add('    <windowsSettings>')
$lines.Add('      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>')
$lines.Add('      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>')
$lines.Add('    </windowsSettings>')
$lines.Add('  </application>')

foreach ($fragment in $fragments) {
    [xml] $xml = Get-Content -Raw -LiteralPath $fragment
    $namespace = [Xml.XmlNamespaceManager]::new($xml.NameTable)
    $namespace.AddNamespace("m", "http://schemas.microsoft.com/appx/manifest/foundation/windows10")

    foreach ($server in $xml.SelectNodes("/m:Fragment/m:Extensions/m:Extension/m:InProcessServer", $namespace)) {
        $dll = [Security.SecurityElement]::Escape($server.Path)
        $lines.Add("  <asmv3:file name=`"$dll`">")
        foreach ($class in $server.ActivatableClass) {
            $name = [Security.SecurityElement]::Escape($class.ActivatableClassId)
            $lines.Add("    <winrtv1:activatableClass name=`"$name`" threadingModel=`"both`"/>")
        }
        $lines.Add('  </asmv3:file>')
    }

    foreach ($proxy in $xml.SelectNodes("/m:Fragment/m:Extensions/m:Extension/m:ProxyStub", $namespace)) {
        $dll = [string] $proxy.Path
        if ($dll -eq "PushNotificationsLongRunningTask.ProxyStub.dll" -or $dll -eq "Microsoft.Windows.Widgets.dll") {
            continue
        }
        $escapedDll = [Security.SecurityElement]::Escape($dll)
        $classId = [Security.SecurityElement]::Escape([string] $proxy.ClassId)
        $lines.Add("  <asmv3:file name=`"$escapedDll`">")
        $lines.Add("    <asmv3:comClass clsid=`"{$classId}`"/>")
        foreach ($interface in $proxy.Interface) {
            $name = [Security.SecurityElement]::Escape([string] $interface.Name)
            $interfaceId = [Security.SecurityElement]::Escape([string] $interface.InterfaceId)
            $lines.Add("    <asmv3:comInterfaceProxyStub name=`"$name`" iid=`"{$interfaceId}`"/>")
        }
        $lines.Add('  </asmv3:file>')
    }
}

$lines.Add('</assembly>')
Set-Content -LiteralPath $Output -Value $lines -Encoding UTF8
