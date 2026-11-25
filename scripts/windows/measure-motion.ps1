# measure-motion.ps1 - measures how long the panel takes to settle after a swipe.
#
# Swipes with real mouse input from (FromX, Y) to (ToX, Y) in 20 steps, then captures the panel
# window as fast as it can for 700 ms after the mouse up. A frame counts as settled when it matches
# the last one at a grid of sampled pixels. The script prints the number of frames and how long
# after the mouse up the panel settled, saves the last frame as motion-settled.png in the captures
# folder ($env:PLINTH_SHOTS, or $env:TEMP\plinth-shots), and then swipes back towards page 1.
#
# Windows PowerShell 5.1, from the repo root, with the panel on page 1:
#   powershell -ExecutionPolicy Bypass -File scripts\windows\measure-motion.ps1
#       page 1 to page 2 (the defaults: -FromX 395 -ToX 60 -Y 150)
#   powershell -ExecutionPolicy Bypass -File scripts\windows\measure-motion.ps1 -FromX 300 -ToX 700
#       a swipe towards the previous page, which the first page refuses
#
# Coordinates are physical pixels relative to the panel window's top-left corner (see
# panel-driver.ps1). Do not touch the mouse while it runs.
param([int]$FromX = 395, [int]$ToX = 60, [int]$Y = 150)
. "$PSScriptRoot\panel-driver.ps1"
$S = $script:Shots
$r = Get-PanelRect; $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
[PD]::SetCursorPos($r.Left+$FromX,$r.Top+$Y)|Out-Null; Start-Sleep -Milliseconds 80; [PD]::mouse_event(2,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 30
for($i=1;$i -le 20;$i++){ [PD]::SetCursorPos($r.Left+$FromX+($ToX-$FromX)*$i/20,$r.Top+$Y)|Out-Null; Start-Sleep -Milliseconds 12 }
[PD]::mouse_event(4,0,0,0,[UIntPtr]::Zero); $sw=[Diagnostics.Stopwatch]::StartNew(); $frames=@()
while($sw.ElapsedMilliseconds -lt 700){ $bmp=New-Object System.Drawing.Bitmap $w,$h; $g=[System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($r.Left,$r.Top,0,0,$bmp.Size); $frames += ,@($sw.ElapsedMilliseconds,$bmp); $g.Dispose() }
# A frame is "settled" when it equals the last frame at sampled pixels
$last = $frames[-1][1]
function Same($a,$b){ for($x=140;$x -lt 900;$x+=23){ for($y=10;$y -lt 300;$y+=17){ if($a.GetPixel($x,$y) -ne $b.GetPixel($x,$y)){ return $false } } }; $true }
$settledAt = $null; for($k=$frames.Count-1;$k -ge 0;$k--){ if(Same $frames[$k][1] $last){ $settledAt=$frames[$k][0] } else { break } }
"frames: $($frames.Count), first change frame ms: $($frames[0][0]), settled from ~${settledAt} ms after mouse up"
$last.Save("$S\motion-settled.png"); foreach($f in $frames){ $f[1].Dispose() }
"foreground='$(Get-Foreground)'"
Invoke-PanelDrag 300 150 700 150 20; Save-PanelShot 'd06f-back-page1'
