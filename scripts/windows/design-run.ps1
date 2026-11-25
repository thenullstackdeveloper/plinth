# design-run.ps1 - runs points 4 to 8 of the design test in docs/flow/2026-11-20-handoff-windows.md
# with real mouse input: a button held and released, neighbouring buttons in a row, swipes from a
# gap, from below the buttons and from a button, the lock, and drags across the sidebar. Every
# capture goes to the panel-driver.ps1 folder ($env:PLINTH_SHOTS, or $env:TEMP\plinth-shots).
#
# Windows PowerShell 5.1, from the repo root:
#   powershell -ExecutionPolicy Bypass -File scripts\windows\design-run.ps1
#
# Start the panel first, then a Notepad with a new empty file (it stands in for the game and holds
# the foreground), then this script. Do not touch the mouse while it runs.
#
# The coordinates are measured by hand on captures of one layout: the wow-default-keys template,
# the sidebar on the left and a 916x309 window, in physical pixels. With another template, side or
# size they land somewhere else. A 100% display scale was deduced from that size, not measured.
. "$PSScriptRoot\panel-driver.ps1"
$S = $script:Shots
function Save-ForegroundShot([string]$name) {
  $h = [PD]::GetForegroundWindow(); $r = New-Object PDRECT; [PD]::GetWindowRect($h, [ref]$r) | Out-Null
  $bmp = New-Object System.Drawing.Bitmap 600, 160; $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size); $bmp.Save((Join-Path $S "$name.png")); $g.Dispose(); $bmp.Dispose()
  "[$name] foreground window '$(Get-Foreground)'"
}
function Move-To([int]$x, [int]$y) { $r = Get-PanelRect; [PD]::SetCursorPos($r.Left + $x, $r.Top + $y) | Out-Null; Start-Sleep -Milliseconds 80 }
function Down { [PD]::mouse_event(0x2, 0, 0, 0, [UIntPtr]::Zero) }
function Up { [PD]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero) }
# Burst of captures right after a swipe, with timestamps, to see the slide.
function Invoke-DragBurst([int]$x1, [int]$y1, [int]$x2, [int]$y2, [string]$name) {
  $r = Get-PanelRect; $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  Move-To $x1 $y1; Down; Start-Sleep -Milliseconds 30
  for ($i = 1; $i -le 20; $i++) { [PD]::SetCursorPos($r.Left + $x1 + ($x2-$x1)*$i/20, $r.Top + $y1 + ($y2-$y1)*$i/20) | Out-Null; Start-Sleep -Milliseconds 12 }
  Up; $sw = [Diagnostics.Stopwatch]::StartNew(); $log = @()
  for ($k = 0; $k -lt 10; $k++) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h; $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size); $t = $sw.ElapsedMilliseconds
    $bmp.Save((Join-Path $S ("{0}-f{1:d2}-{2}ms.png" -f $name, $k, $t))); $g.Dispose(); $bmp.Dispose(); $log += "$t"
  }
  Start-Sleep -Milliseconds 400; "[$name] burst at ms: $($log -join ',')  foreground='$(Get-Foreground)'"
}

# --- point 4: hold Action 1, capture while pressed, release
Move-To 270 80; Down; Start-Sleep -Milliseconds 300; Save-PanelShot 'd04-pressed-action1'
Up; Start-Sleep -Milliseconds 400; Save-PanelShot 'd04-released-action1'
Save-ForegroundShot 'd04-notepad'
# --- point 5: Action 1, Action 2, Action 4 in a row
Invoke-PanelClick 270 80; Invoke-PanelClick 521 80; Invoke-PanelClick 270 222
Save-ForegroundShot 'd05-notepad'
# --- point 6: swipe from the gap between buttons (to page 2), from below the buttons (back), from a button (to page 2)
Invoke-DragBurst 395 150 60 150 'd06a-gap-next'
Invoke-DragBurst 300 295 700 295 'd06b-below-prev'
Invoke-DragBurst 773 80 300 80 'd06c-onbutton-next'
Save-ForegroundShot 'd06-notepad'
Invoke-PanelDrag 300 150 700 150 20; Save-PanelShot 'd06d-back-page1'
# --- point 7: Lock, then Unlock
Invoke-PanelClick 71 53; Save-PanelShot 'd07-locked'
Invoke-PanelDrag 600 150 200 150 20; Save-PanelShot 'd07-locked-swipe'
Invoke-PanelClick 71 53; Save-PanelShot 'd07-unlocked'
# --- point 8: drag across the sidebar, from gap to gap, over the labels
Invoke-PanelDrag 70 101 70 199 20; Save-PanelShot 'd08-sidebar-drag-vertical'
Invoke-PanelDrag 12 60 130 150 20; Save-PanelShot 'd08-sidebar-drag-diagonal'
Save-ForegroundShot 'd08-notepad'
