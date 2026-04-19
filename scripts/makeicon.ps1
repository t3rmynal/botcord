$signature = @'
using System;
using System.Runtime.InteropServices;

public static class NativeIcon {
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern int ExtractIconExW(string lpszFile, int nIconIndex, IntPtr[] phiconLarge, IntPtr[] phiconSmall, uint nIcons);

    [DllImport("user32.dll")]
    public static extern bool DestroyIcon(IntPtr handle);
}
'@
Add-Type -TypeDefinition $signature

Add-Type -AssemblyName System.Drawing

function Save-IconFromDll {
    param(
        [string] $Dll,
        [int]    $Index,
        [string] $PngOut,
        [string] $IcoOut
    )

    $large = New-Object IntPtr[] 1
    $small = New-Object IntPtr[] 1
    $n = [NativeIcon]::ExtractIconExW($Dll, $Index, $large, $small, 1)
    if ($n -lt 1 -or $large[0] -eq [IntPtr]::Zero) {
        throw "could not extract icon $Index from $Dll"
    }

    $icon = [System.Drawing.Icon]::FromHandle($large[0])

    $fs = [System.IO.File]::Create($IcoOut)
    $icon.Save($fs)
    $fs.Close()

    $bmp = $icon.ToBitmap()
    $scaled = New-Object System.Drawing.Bitmap 256, 256
    $g = [System.Drawing.Graphics]::FromImage($scaled)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($bmp, 0, 0, 256, 256)
    $g.Dispose()
    $scaled.Save($PngOut, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $scaled.Dispose()
    $icon.Dispose()

    [NativeIcon]::DestroyIcon($large[0]) | Out-Null
    if ($small[0] -ne [IntPtr]::Zero) { [NativeIcon]::DestroyIcon($small[0]) | Out-Null }
}

$base = 'C:\Users\t3rmynal\Documents\repos-src\botcord\src-tauri\icons'
$png  = Join-Path $base 'icon.png'
$ico  = Join-Path $base 'icon.ico'

$dll = Join-Path $env:WINDIR 'System32\imageres.dll'
Save-IconFromDll -Dll $dll -Index 11 -PngOut $png -IcoOut $ico

Write-Host "wrote $ico and $png"
