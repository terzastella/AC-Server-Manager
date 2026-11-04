# Genera gli asset del brand in modo nitido:
#  - app/public/logo-*-mark.svg   (logo interfaccia, vettoriale)
#  - docs/brand/logo-*.ico        (icone exe, frame 16..256 renderizzati per dimensione)
#  - docs/brand/mark-*.png        (master 1024 per documentazione)
#  - app/src-tauri/icons/*.png    (icone richieste da Tauri)
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File .\generate-brand.ps1
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$here = $PSScriptRoot
$repo = Split-Path -Parent (Split-Path -Parent $here)
$public = Join-Path $repo "app\public"
$iconsDir = Join-Path $repo "app\src-tauri\icons"

$familyName = $null
foreach ($name in @("Bahnschrift", "Segoe UI Variable Display", "Segoe UI")) {
    try {
        $family = New-Object System.Drawing.FontFamily $name
        $familyName = $name
        break
    } catch { }
}
if (-not $familyName) { throw "Nessun font disponibile per il logo." }
Write-Host "Font logo: $familyName"

function New-AcPath([single]$em) {
    $gp = New-Object System.Drawing.Drawing2D.GraphicsPath
    # Winding: i contorni del glifo (es. la A) si sovrappongono e con Alternate
    # comparirebbero dei buchi bianchi sulla traversa.
    $gp.FillMode = [System.Drawing.Drawing2D.FillMode]::Winding
    $fmt = New-Object System.Drawing.StringFormat
    $gp.AddString("AC", $family, [int][System.Drawing.FontStyle]::Bold, $em, (New-Object System.Drawing.PointF 0, 0), $fmt)
    $gp
}

# Converte il contorno GDI+ in comandi SVG, applicando scala e traslazione.
function Convert-ToSvgPath($gp, [double]$scale, [double]$dx, [double]$dy) {
    $pts = $gp.PathPoints
    $types = $gp.PathTypes
    $ci = [System.Globalization.CultureInfo]::InvariantCulture
    $sb = New-Object System.Text.StringBuilder
    $i = 0
    while ($i -lt $pts.Count) {
        $kind = $types[$i] -band 0x07
        $closes = ($types[$i] -band 0x80) -ne 0
        $x = [math]::Round(($pts[$i].X * $scale) + $dx, 2).ToString($ci)
        $y = [math]::Round(($pts[$i].Y * $scale) + $dy, 2).ToString($ci)
        switch ($kind) {
            0 { [void]$sb.Append("M$x $y") }
            1 { [void]$sb.Append("L$x $y") }
            3 {
                $x1 = [math]::Round(($pts[$i].X * $scale) + $dx, 2).ToString($ci)
                $y1 = [math]::Round(($pts[$i].Y * $scale) + $dy, 2).ToString($ci)
                $x2 = [math]::Round(($pts[$i + 1].X * $scale) + $dx, 2).ToString($ci)
                $y2 = [math]::Round(($pts[$i + 1].Y * $scale) + $dy, 2).ToString($ci)
                $x3 = [math]::Round(($pts[$i + 2].X * $scale) + $dx, 2).ToString($ci)
                $y3 = [math]::Round(($pts[$i + 2].Y * $scale) + $dy, 2).ToString($ci)
                [void]$sb.Append("C$x1 $y1 $x2 $y2 $x3 $y3")
                if (($types[$i + 2] -band 0x80) -ne 0) { [void]$sb.Append("Z") }
                $i += 2
            }
        }
        if ($kind -ne 3 -and $closes) { [void]$sb.Append("Z") }
        $i++
    }
    $sb.ToString()
}

$gp = New-AcPath 200
$bounds = $gp.GetBounds()
$width = 264.0
$scale = $width / $bounds.Width
$cap = $bounds.Height * $scale

$managerAccent = "#ff894b"
$launcherAccent = "#43c8db"

Write-Host "Contorni AC pronti (cap=$([math]::Round($cap,1)), larghezza=$width)."

# Badge per l'interfaccia: riquadro arrotondato + AC + accento, stessa famiglia dell'icona exe.
function New-Badge([string]$accent, [string]$id) {
    $tile = 100.0
    $acW = $tile * 0.56
    $s = $acW / $width          # i path AC sono normalizzati su 264 di larghezza
    $acH = $cap * $s
    $acX = [math]::Round(($tile - $acW) / 2, 2)
    $acY = [math]::Round(($tile - $acH) / 2 - 4, 2)
    $barW = [math]::Round($tile * 0.40, 2)
    $barY = [math]::Round($acY + $acH + 7, 2)
    $barH = [math]::Round($tile * 0.055, 2)
    $barX = [math]::Round(($tile - $barW) / 2, 2)
    $path = Convert-ToSvgPath $gp $s (-$bounds.X * $s) (-$bounds.Y * $s)
    @"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <defs>
    <linearGradient id="$id" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#202730"/>
      <stop offset="1" stop-color="#0e1217"/>
    </linearGradient>
  </defs>
  <rect x="1" y="1" width="98" height="98" rx="26" fill="url(#$id)" stroke="#5a6878" stroke-opacity="0.35" stroke-width="2"/>
  <g transform="translate($acX $acY)">
    <path fill="#e0e6ec" d="$path"/>
  </g>
  <rect x="$barX" y="$barY" width="$barW" height="$barH" rx="$([math]::Round($barH / 2, 2))" fill="$accent"/>
</svg>
"@
}

Set-Content -Path (Join-Path $public "logo-manager-badge.svg") -Value (New-Badge $managerAccent "badgeManager") -Encoding UTF8
Set-Content -Path (Join-Path $public "logo-launcher-badge.svg") -Value (New-Badge $launcherAccent "badgeLauncher") -Encoding UTF8
Write-Host "Badge SVG scritti."

# ---------- icone exe: tile arrotondata + AC + accento, render per dimensione ----------
function New-RoundedRect([single]$x, [single]$y, [single]$w, [single]$h, [single]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    $p
}

function Render-Icon([int]$size, [System.Drawing.Color]$accent) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)

    $pad = [single]($size * 0.02)
    $w = [single]($size - (2 * $pad))
    $r = [single]($size * 0.22)
    $tile = New-RoundedRect $pad $pad $w $w $r
    $c1 = [System.Drawing.Color]::FromArgb(255, 30, 37, 46)
    $c2 = [System.Drawing.Color]::FromArgb(255, 11, 14, 18)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush((New-Object System.Drawing.PointF 0, $pad), (New-Object System.Drawing.PointF 0, ($pad + $w)), $c1, $c2)
    $g.FillPath($brush, $tile)
    $brush.Dispose()
    $penW = [math]::Max(1, $size / 64)
    $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(90, 70, 84, 100)), $penW
    $g.DrawPath($pen, $tile)
    $pen.Dispose()

    # algebra identica all'SVG, normalizzata sulla larghezza utile
    $acW = $w * 0.74
    $s = $acW / $bounds.Width
    $acH = $bounds.Height * $s
    $left = $pad + (($w - $acW) / 2)
    $top = $pad + (($w - $acH) * 0.42)
    $state = $g.Save()
    $g.TranslateTransform([single]$left, [single]$top)
    $g.ScaleTransform([single]$s, [single]$s)
    $g.TranslateTransform([single](-$bounds.X), [single](-$bounds.Y))
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 224, 230, 236))
    $g.FillPath($white, $gp)
    $white.Dispose()
    $g.Restore($state)

    # barra accento
    if ($size -ge 24) {
        $barW = [single]($w * 0.46)
        $barH = [single][math]::Max(1.5, $size * 0.045)
        $barY = [single]($pad + ($w * 0.76))
        $bar = New-RoundedRect ([single]($pad + (($w - $barW) / 2))) $barY $barW $barH ([single]($barH / 2))
        $abrush = New-Object System.Drawing.SolidBrush $accent
        $g.FillPath($abrush, $bar)
        $abrush.Dispose()
        $bar.Dispose()
    }

    $tile.Dispose()
    $g.Dispose()
    $bmp
}

function Save-Ico([string]$path, [int[]]$sizes, [System.Drawing.Color]$accent, [int]$keepUpTo) {
    $frames = @()
    foreach ($s in $sizes) {
        if ($s -gt $keepUpTo) { continue }
        $bmp = Render-Icon $s $accent
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $frames += , @{ Size = $s; Data = $ms.ToArray() }
        $ms.Dispose()
        $bmp.Dispose()
    }
    $fs = [System.IO.File]::Create($path)
    $bw = New-Object System.IO.BinaryWriter $fs
    $bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$frames.Count)
    $offset = 6 + (16 * $frames.Count)
    foreach ($f in $frames) {
        $dim = if ($f.Size -ge 256) { 0 } else { $f.Size }
        $bw.Write([byte]$dim); $bw.Write([byte]$dim)
        $bw.Write([byte]0); $bw.Write([byte]0)
        $bw.Write([uint16]1); $bw.Write([uint16]32)
        $bw.Write([uint32]$f.Data.Length); $bw.Write([uint32]$offset)
        $offset += $f.Data.Length
    }
    foreach ($f in $frames) { $bw.Write($f.Data) }
    $bw.Flush(); $bw.Close(); $fs.Close()
    Write-Host "ICO $([System.IO.Path]::GetFileName($path)): $($frames.Count) frame"
}

$all = @(16, 20, 24, 32, 40, 48, 64, 128, 256)
$manager = [System.Drawing.Color]::FromArgb(255, 255, 137, 75)
$launcher = [System.Drawing.Color]::FromArgb(255, 67, 200, 219)
Save-Ico (Join-Path $here "logo-manager.ico") $all $manager 256
Save-Ico (Join-Path $here "logo-launcher.ico") $all $launcher 256

# master PNG per documentazione
foreach ($pair in @(@("mark-manager", $manager), @("mark-launcher", $launcher))) {
    $bmp = Render-Icon 512 $pair[1]
    $bmp.Save((Join-Path $here "$($pair[0]).png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

# icone richieste da Tauri
$tauriIcons = @{
    "32x32.png"       = 32
    "64x64.png"       = 64
    "128x128.png"     = 128
    "128x128@2x.png"  = 256
    "icon.png"        = 512
}
foreach ($key in $tauriIcons.Keys) {
    $bmp = Render-Icon $tauriIcons[$key] $manager
    $bmp.Save((Join-Path $iconsDir $key), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}
$bmp = Render-Icon 256 $launcher
$bmp.Save((Join-Path $iconsDir "launcher.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Host "Fatto."
