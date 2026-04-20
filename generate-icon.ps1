# Win脚本中心 - 图标生成脚本
# 使用 PowerShell 生成一个简单的应用图标
# 运行方式: 右键 -> 使用 PowerShell 运行

$iconDir = "d:\KaiFa\YuanMa\HouDuan\yx-launch-Platform\win-script-hub\icons"
New-Item -ItemType Directory -Path $iconDir -Force | Out-Null

Add-Type -AssemblyName System.Drawing

$sizes = @(16, 32, 48, 64, 128, 256)
$images = @()

foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'HighQuality'
    $g.InterpolationMode = 'HighQualityBicubic'
    
    # 背景渐变
    $rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $rect,
        [System.Drawing.Color]::FromArgb(37, 99, 235),   # Blue-600
        [System.Drawing.Color]::FromArgb(118, 75, 162)   # Purple-600
    )
    $g.FillRectangle($brush, $rect)
    
    # 绘制闪电符号
    $fontSize = [int]($size * 0.6)
    $font = New-Object System.Drawing.Font("Segoe UI", $fontSize, [System.Drawing.FontStyle]::Bold)
    $textBrush = [System.Drawing.Brushes]::White
    $sf = New-Object System.Drawing.StringFormat
    $sf.Alignment = 'Center'
    $sf.LineAlignment = 'Center'
    $textRect = New-Object System.Drawing.RectangleF(0, 0, $size, $size)
    $g.DrawString("⚡", $font, $textBrush, $textRect, $sf)
    
    $g.Dispose()
    $images += $bmp
}

# 保存 256x256 PNG
$pngPath = Join-Path $iconDir "icon.png"
$images[-1].Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)

# 生成 ICO 文件（使用多个尺寸）
$icoPath = Join-Path $iconDir "icon.ico"
$ms = New-Object System.IO.MemoryStream

# ICO 文件头
$writer = New-Object System.IO.BinaryWriter($ms)
$writer.Write([Int16]0)          # Reserved
$writer.Write([Int16]1)          # ICO type
$writer.Write([Int16]$sizes.Count)  # Number of images

# 计算偏移量
$headerSize = 6 + (16 * $sizes.Count)
$offset = $headerSize
$imageData = @()

foreach ($i in 0..($sizes.Count - 1)) {
    $bmp = $images[$i]
    $imgMs = New-Object System.IO.MemoryStream
    $bmp.Save($imgMs, [System.Drawing.Imaging.ImageFormat]::Png)
    $imgBytes = $imgMs.ToArray()
    $imageData += ,$imgBytes
    
    # ICO 目录项
    $sizeVal = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $writer.Write([Byte]$sizeVal)      # Width
    $writer.Write([Byte]$sizeVal)      # Height
    $writer.Write([Byte]0)             # Color palette
    $writer.Write([Byte]0)             # Reserved
    $writer.Write([Int16]1)            # Color planes
    $writer.Write([Int16]32)           # Bits per pixel
    $writer.Write([Int32]$imgBytes.Length)  # Image size
    $writer.Write([Int32]$offset)      # Offset
    
    $offset += $imgBytes.Length
    $imgMs.Dispose()
}

# 写入图像数据
foreach ($imgBytes in $imageData) {
    $writer.Write($imgBytes)
}

# 保存 ICO
[System.IO.File]::WriteAllBytes($icoPath, $ms.ToArray())
$writer.Dispose()
$ms.Dispose()

# 清理
foreach ($bmp in $images) { $bmp.Dispose() }

Write-Host "图标已生成:"
Write-Host "  PNG: $pngPath"
Write-Host "  ICO: $icoPath"
