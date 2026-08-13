//! Phase 0 focus spike for Windows.
//!
//! Opens a small always-on-top window created with `WS_EX_NOACTIVATE` - the
//! Windows equivalent of Hyprland's `nofocus`. Clicking it must NOT take
//! keyboard focus away from the game.
//!
//! On every click the window:
//!   1. asks Windows which window currently holds focus (`GetForegroundWindow`)
//!      and prints its title - this is the evidence for the R1 test, replacing
//!      `hyprctl activewindow` from the Linux plan;
//!   2. emits one key press through the real `WindowsInputSender` adapter.
//!
//! If the reported foreground window is still Star Citizen *and* the key lands
//! in the game, the critical risk of the project is retired.
//!
//! Usage: spike_win.exe [keycode] [x] [y] [width] [height]
//!   keycode defaults to 30 (KEY_A). Use 59 for F1, 38 for L.
//!   x/y let you place the panel on a second monitor without dragging it.

#[cfg(target_os = "windows")]
fn main() {
    windows_spike::run();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("spike_win is Windows-only: it tests WS_EX_NOACTIVATE focus behaviour.");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
mod windows_spike {
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Mutex;

    use plinth_application::{InputSender, KeyEvent};
    use plinth_domain::ScanCode;
    use plinth_infra::input_windows::WindowsInputSender;

    use windows::core::w;
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DrawTextW, EndPaint, InvalidateRect, SetBkMode, SetTextColor,
        DT_CENTER, DT_WORDBREAK, PAINTSTRUCT, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
        GetWindowTextW, LoadCursorW, PostQuitMessage, RegisterClassW, ShowWindow, HMENU, IDC_ARROW,
        MSG, SW_SHOWNOACTIVATE, WM_DESTROY, WM_LBUTTONDOWN, WM_PAINT, WNDCLASSW, WS_EX_NOACTIVATE,
        WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    };

    static KEYCODE: AtomicU16 = AtomicU16::new(30);
    static STATUS: Mutex<String> = Mutex::new(String::new());

    fn set_status(text: String) {
        println!("{text}");
        if let Ok(mut guard) = STATUS.lock() {
            *guard = text;
        }
    }

    /// Reads the title of whatever window currently has focus.
    fn foreground_window_title() -> String {
        // SAFETY: plain Win32 queries with a correctly sized buffer.
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return "<none>".to_string();
            }
            let mut buffer = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut buffer);
            if len <= 0 {
                return "<untitled>".to_string();
            }
            String::from_utf16_lossy(&buffer[..len as usize])
        }
    }

    fn on_click() {
        let focus = foreground_window_title();
        let code = ScanCode::new(KEYCODE.load(Ordering::Relaxed));

        let sender = WindowsInputSender::new();
        let result = sender.emit(&[KeyEvent::down(code), KeyEvent::up(code)]);

        let outcome = match result {
            Ok(()) => format!("sent scancode {}", code.get()),
            Err(e) => format!("emit FAILED: {e}"),
        };
        set_status(format!("focus: {focus}  |  {outcome}"));
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_LBUTTONDOWN => {
                on_click();
                // SAFETY: valid window handle owned by this thread.
                unsafe {
                    let _ = InvalidateRect(Some(hwnd), None, true);
                }
                LRESULT(0)
            }
            WM_PAINT => {
                // SAFETY: standard paint cycle, ps is initialised by BeginPaint.
                unsafe {
                    let mut ps = PAINTSTRUCT::default();
                    let hdc = BeginPaint(hwnd, &mut ps);

                    SetBkMode(hdc, TRANSPARENT);
                    SetTextColor(hdc, COLORREF(0x00E1_ECF8));

                    let mut rect = RECT {
                        left: 16,
                        top: 16,
                        right: ps.rcPaint.right - 16,
                        bottom: ps.rcPaint.bottom - 16,
                    };
                    let status = STATUS
                        .lock()
                        .map(|s| s.clone())
                        .unwrap_or_else(|_| String::new());
                    let text = if status.is_empty() {
                        "PLINTH SPIKE\n\nClick me while the game is focused.".to_string()
                    } else {
                        format!("PLINTH SPIKE\n\n{status}")
                    };
                    let mut wide: Vec<u16> = text.encode_utf16().collect();
                    DrawTextW(hdc, &mut wide, &mut rect, DT_CENTER | DT_WORDBREAK);

                    let _ = EndPaint(hwnd, &ps);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // SAFETY: ends the message loop.
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            // SAFETY: default handling for everything else.
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    pub fn run() {
        let mut args = std::env::args().skip(1);
        let code: u16 = args.next().and_then(|a| a.parse().ok()).unwrap_or(30);
        let x: i32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(200);
        let y: i32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(200);
        let width: i32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(560);
        let height: i32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(220);
        KEYCODE.store(code, Ordering::Relaxed);

        println!("Plinth spike window - scancode {code} at {x},{y} ({width}x{height})");
        println!("Focus the game, then click this window. It must NOT steal focus.");

        // SAFETY: standard Win32 window creation and message pump.
        unsafe {
            let instance = GetModuleHandleW(None).expect("GetModuleHandleW");
            let class_name = w!("PlinthSpikeWindow");

            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW).expect("LoadCursorW"),
                hbrBackground: CreateSolidBrush(COLORREF(0x001A_1611)),
                ..Default::default()
            };
            let atom = RegisterClassW(&wc);
            assert!(atom != 0, "RegisterClassW failed");

            let hwnd = CreateWindowExW(
                // NOACTIVATE is the whole point: the window must never take focus.
                WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                class_name,
                w!("Plinth"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                width,
                height,
                None,
                None::<HMENU>,
                Some(instance.into()),
                None,
            )
            .expect("CreateWindowExW");

            // SHOWNOACTIVATE displays the window without activating it.
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                DispatchMessageW(&message);
            }
        }
    }
}
