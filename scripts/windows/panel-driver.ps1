# panel-driver.ps1 - helpers that drive the Plinth panel on Windows with real OS mouse input.
#
# Windows PowerShell 5.1 (powershell.exe). Load it with dot-source, so that its functions and
# $script:Shots land in the caller's scope:
#   . "$PSScriptRoot\panel-driver.ps1"
#
# Functions:
#   Get-PanelRect                          the panel window's rectangle (Left, Top, Right, Bottom).
#   Get-Foreground                         the title of the window that holds the foreground,
#                                          that is, the keyboard focus.
#   Invoke-PanelClick x y                  moves the cursor to x, y, clicks the left button and
#                                          waits 400 ms.
#   Invoke-PanelDrag x1 y1 x2 y2 [steps]   presses at x1, y1, moves to x2, y2 in `steps` steps
#                                          (20 by default), releases and waits 600 ms.
#   Save-PanelShot name                    captures the panel window to <shots>\name.png and
#                                          prints the foreground title and the panel's position
#                                          and size.
#
# Coordinates are physical pixels relative to the panel window's top-left corner: the script
# calls SetProcessDPIAware when it loads, so GetWindowRect, SetCursorPos and CopyFromScreen all
# work in physical pixels, whatever the display scale.
#
# The window is found by process: the first plinth-tauri process that has a main window.
#
# Captures go to $env:PLINTH_SHOTS if it is set and to $env:TEMP\plinth-shots otherwise; the
# folder is created if it does not exist.
#
# The mouse belongs to the script: do not touch it while a run is going, or the moves and clicks
# land somewhere else.
Add-Type -AssemblyName System.Drawing
if (-not ('PD' -as [type])) {
Add-Type @'
using System; using System.Text; using System.Runtime.InteropServices;
public struct PDRECT { public int Left, Top, Right, Bottom; }
public static class PD {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out PDRECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
}
'@
}
[PD]::SetProcessDPIAware() | Out-Null
$script:Shots = $env:PLINTH_SHOTS
if (-not $script:Shots) { $script:Shots = Join-Path $env:TEMP 'plinth-shots' }
New-Item -ItemType Directory -Force -Path $script:Shots | Out-Null

function Get-PanelRect {
  $p = Get-Process plinth-tauri -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  $r = New-Object PDRECT; [PD]::GetWindowRect($p.MainWindowHandle, [ref]$r) | Out-Null; $r
}
function Get-Foreground {
  $sb = New-Object System.Text.StringBuilder 256
  [PD]::GetWindowText([PD]::GetForegroundWindow(), $sb, 256) | Out-Null; $sb.ToString()
}
# x, y are relative to the panel window's top-left corner.
function Invoke-PanelClick([int]$x, [int]$y) {
  $r = Get-PanelRect
  [PD]::SetCursorPos($r.Left + $x, $r.Top + $y) | Out-Null; Start-Sleep -Milliseconds 80
  [PD]::mouse_event(0x2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60
  [PD]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 400
}
function Invoke-PanelDrag([int]$x1, [int]$y1, [int]$x2, [int]$y2, [int]$steps = 20) {
  $r = Get-PanelRect
  [PD]::SetCursorPos($r.Left + $x1, $r.Top + $y1) | Out-Null; Start-Sleep -Milliseconds 80
  [PD]::mouse_event(0x2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
  for ($i = 1; $i -le $steps; $i++) {
    $x = $x1 + ($x2 - $x1) * $i / $steps; $y = $y1 + ($y2 - $y1) * $i / $steps
    [PD]::SetCursorPos($r.Left + [int]$x, $r.Top + [int]$y) | Out-Null; Start-Sleep -Milliseconds 12
  }
  [PD]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 600
}
function Save-PanelShot([string]$name) {
  $r = Get-PanelRect; $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h; $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
  $out = Join-Path $script:Shots "$name.png"; $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  "[$name] foreground='$(Get-Foreground)' panel=$($r.Left),$($r.Top) ${w}x${h}"
}
