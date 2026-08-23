use tauri::AppHandle;

#[tauri::command]
pub fn begin_native_volume_drag(app: AppHandle, drag_id: u64) -> Result<bool, String> {
    platform::begin(app, drag_id)
}

#[tauri::command]
pub fn end_native_volume_drag(drag_id: u64) -> Result<(), String> {
    platform::end(drag_id)
}

#[cfg(target_os = "windows")]
mod platform {
    use std::sync::{mpsc, Mutex, OnceLock};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use serde::Serialize;
    use tauri::{AppHandle, Emitter};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetCursorPos, GetMessageW, PeekMessageW, PostThreadMessageW,
        SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, MSG, MSLLHOOKSTRUCT, PM_NOREMOVE,
        WH_MOUSE_LL, WM_LBUTTONUP, WM_MOUSEMOVE, WM_QUIT, WM_USER,
    };

    const VOLUME_DRAG_EVENT: &str = "native-volume-drag";

    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct VolumeDragDelta {
        drag_id: u64,
        delta_y: i32,
    }

    struct HookState {
        anchor: POINT,
        delta_sender: mpsc::Sender<i32>,
        drag_id: u64,
    }

    struct DragSession {
        drag_id: u64,
        hook_thread: JoinHandle<()>,
        thread_id: u32,
    }

    static HOOK_STATE: OnceLock<Mutex<Option<HookState>>> = OnceLock::new();
    static DRAG_SESSION: OnceLock<Mutex<Option<DragSession>>> = OnceLock::new();

    fn hook_state() -> &'static Mutex<Option<HookState>> {
        HOOK_STATE.get_or_init(|| Mutex::new(None))
    }

    fn drag_session() -> &'static Mutex<Option<DragSession>> {
        DRAG_SESSION.get_or_init(|| Mutex::new(None))
    }

    pub fn begin(app: AppHandle, drag_id: u64) -> Result<bool, String> {
        end_current()?;

        let mut anchor = POINT::default();
        // SAFETY: `anchor` points to initialized, writable memory for the duration of the call.
        if unsafe { GetCursorPos(&mut anchor) } == 0 {
            return Err("カーソル位置を取得できませんでした".to_string());
        }

        let (delta_sender, delta_receiver) = mpsc::channel::<i32>();
        *hook_state()
            .lock()
            .map_err(|_| "マウスフックの状態をロックできませんでした".to_string())? =
            Some(HookState {
                anchor,
                delta_sender,
                drag_id,
            });

        thread::spawn(move || {
            while let Ok(mut delta_y) = delta_receiver.recv() {
                // 高ポーリングレートのマウスでも WebView をイベントまみれにしないよう、
                // 約 120 Hz ごとに移動量をまとめる。
                thread::sleep(Duration::from_millis(8));
                delta_y += delta_receiver.try_iter().sum::<i32>();
                let _ = app.emit(VOLUME_DRAG_EVENT, VolumeDragDelta { drag_id, delta_y });
            }
        });

        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let hook_thread = thread::spawn(move || run_hook(ready_sender));
        let thread_id = match ready_receiver.recv() {
            Ok(Ok(thread_id)) => thread_id,
            Ok(Err(error)) => {
                let _ = hook_thread.join();
                clear_hook_state(drag_id);
                return Err(error);
            }
            Err(_) => {
                let _ = hook_thread.join();
                clear_hook_state(drag_id);
                return Err("マウスフックを開始できませんでした".to_string());
            }
        };

        *drag_session()
            .lock()
            .map_err(|_| "ドラッグ状態をロックできませんでした".to_string())? = Some(DragSession {
            drag_id,
            hook_thread,
            thread_id,
        });

        Ok(true)
    }

    pub fn end(drag_id: u64) -> Result<(), String> {
        let session = {
            let mut session = drag_session()
                .lock()
                .map_err(|_| "ドラッグ状態をロックできませんでした".to_string())?;
            if session
                .as_ref()
                .is_some_and(|session| session.drag_id == drag_id)
            {
                session.take()
            } else {
                None
            }
        };

        stop_session(session)?;
        clear_hook_state(drag_id);
        Ok(())
    }

    fn end_current() -> Result<(), String> {
        let session = drag_session()
            .lock()
            .map_err(|_| "ドラッグ状態をロックできませんでした".to_string())?
            .take();
        let drag_id = session.as_ref().map(|session| session.drag_id);
        stop_session(session)?;
        if let Some(drag_id) = drag_id {
            clear_hook_state(drag_id);
        }
        Ok(())
    }

    fn stop_session(session: Option<DragSession>) -> Result<(), String> {
        if let Some(session) = session {
            // SAFETY: The thread id belongs to the hook thread and the arguments are plain values.
            unsafe {
                PostThreadMessageW(session.thread_id, WM_QUIT, 0, 0);
            }
            session
                .hook_thread
                .join()
                .map_err(|_| "マウスフックを終了できませんでした".to_string())?;
        }
        Ok(())
    }

    fn clear_hook_state(drag_id: u64) {
        if let Ok(mut state) = hook_state().lock() {
            if state.as_ref().is_some_and(|state| state.drag_id == drag_id) {
                *state = None;
            }
        }
    }

    fn run_hook(ready_sender: mpsc::SyncSender<Result<u32, String>>) {
        // SAFETY: The current executable module owns `mouse_hook`, whose ABI and lifetime satisfy
        // the global low-level hook contract.
        let module = unsafe { GetModuleHandleW(std::ptr::null()) };
        let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) };
        if hook.is_null() {
            let _ = ready_sender.send(Err(
                "Windows のマウスフックを登録できませんでした".to_string()
            ));
            return;
        }

        // SAFETY: Called from the hook thread whose id is needed by `PostThreadMessageW`.
        let thread_id = unsafe { GetCurrentThreadId() };
        let mut message = MSG::default();
        // SAFETY: Peeking once guarantees this thread has a message queue before it is exposed.
        unsafe {
            PeekMessageW(
                &mut message,
                std::ptr::null_mut(),
                WM_USER,
                WM_USER,
                PM_NOREMOVE,
            );
        }
        if ready_sender.send(Ok(thread_id)).is_err() {
            // SAFETY: `hook` was returned by `SetWindowsHookExW` above.
            unsafe {
                UnhookWindowsHookEx(hook);
            }
            return;
        }

        // SAFETY: `message` is writable and this thread owns the message loop.
        while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {}

        // SAFETY: `hook` was returned by `SetWindowsHookExW` and is unhooked exactly once here.
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }

    unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && wparam as u32 == WM_MOUSEMOVE {
            // SAFETY: For `WH_MOUSE_LL` and `HC_ACTION`, Windows supplies a valid
            // `MSLLHOOKSTRUCT` pointer for the duration of this callback.
            let mouse = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
            if let Ok(state) = hook_state().lock() {
                if let Some(state) = state.as_ref() {
                    let delta_y = state.anchor.y - mouse.pt.y;
                    if delta_y != 0 {
                        let _ = state.delta_sender.send(delta_y);
                    }
                    // Windows にカーソル移動を渡さず、画面上の位置を固定する。
                    return 1;
                }
            }
        }

        if code == HC_ACTION as i32 && wparam as u32 == WM_LBUTTONUP {
            // PointerUp が WebView に届かない場合でも、マウスを離せばフックを停止する。
            // SAFETY: This callback runs on the hook thread and posts to its own message queue.
            unsafe {
                PostThreadMessageW(GetCurrentThreadId(), WM_QUIT, 0, 0);
            }
        }

        // SAFETY: Required by the hook contract for events this hook does not consume.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use tauri::AppHandle;

    pub fn begin(_app: AppHandle, _drag_id: u64) -> Result<bool, String> {
        Ok(false)
    }

    pub fn end(_drag_id: u64) -> Result<(), String> {
        Ok(())
    }
}
