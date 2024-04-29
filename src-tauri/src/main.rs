// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use once_cell::sync::Lazy;
use std::collections::HashSet;
use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
  Graphics::Gdi::{PtInRect, ScreenToClient},
  UI::WindowsAndMessaging::*,
};

macro_rules! MAKELPARAM {
  ($low:expr, $high:expr) => {
    ((($low & 0xffff) as u32) | (($high & 0xffff) as u32) << 16) as _
  };
}

#[tauri::command]
fn set_ignore_cursor_events(window: tauri::Window, ignore: bool, forward: bool) {
  window.set_ignore_cursor_events(ignore).unwrap(); // 设置窗口样式，包含事件穿透与窗口透明

  let hwnd = {
    let hwnd = window.hwnd().unwrap(); // 此处的 HWND 是 Tauri 定义的
    HWND(hwnd.0) // 需要转换为 Win32 定义的 HWND
  };

  let forward = if ignore { forward } else { false };
  unsafe { set_forward_mouse_messages(hwnd, forward) };
}

static mut MOUSE_HOOK_: Option<HHOOK> = None;
static mut FORWARDING_WINDOWS_: Lazy<HashSet<isize>> = Lazy::new(|| HashSet::new());
unsafe fn set_forward_mouse_messages(hwnd: HWND, forward: bool) {
  // 需要将事件转发设置到浏览器进程上
  // 而目前 Tauri 未提供直接获取浏览器进程句柄的方法
  // 故此处通过获取子进程的方式获取浏览器进程的句柄
  let browser_hwnd = {
    let hwnd = GetWindow(hwnd, GW_CHILD);
    GetWindow(hwnd, GW_CHILD)
  };

  if forward {
    FORWARDING_WINDOWS_.insert(browser_hwnd.0); // 插入

    match MOUSE_HOOK_ {
      Some(_) => {}
      None => {
        MOUSE_HOOK_ =
          Some(SetWindowsHookExW(WH_MOUSE_LL, Some(mousemove_forward), None, 0).unwrap());
      }
    }
  } else {
    FORWARDING_WINDOWS_.remove(&browser_hwnd.0); // 移除

    if FORWARDING_WINDOWS_.len() == 0 {
      match MOUSE_HOOK_ {
        Some(hook) => {
          UnhookWindowsHookEx(hook).unwrap();
          MOUSE_HOOK_ = None;
        }
        None => {}
      }
    }
  }
}

unsafe extern "system" fn mousemove_forward(
  n_code: i32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  if n_code < 0 {
    return CallNextHookEx(None, n_code, w_param, l_param);
  }

  if w_param.0 as u32 == WM_MOUSEMOVE {
    let p = l_param.0 as *const MSLLHOOKSTRUCT; // 强转为 MSLLHOOKSTRUCT 的原始指针
    let p = (*p).pt; // 解引用原始指针，再获取 MSLLHOOKSTRUCT 的 pt 字段

    for &hwnd in FORWARDING_WINDOWS_.iter() {
      let hwnd = HWND(hwnd);

      let mut client_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
      };
      GetClientRect(hwnd, &mut client_rect).unwrap();

      let mut p = p.clone();
      ScreenToClient(hwnd, &mut p).unwrap();

      if PtInRect(&client_rect, p).as_bool() {
        let w = WPARAM(1);
        let l = LPARAM(MAKELPARAM!(p.x, p.y));
        SendMessageW(hwnd, WM_MOUSEMOVE, w, l);
      }
    }
  }

  CallNextHookEx(None, n_code, w_param, l_param)
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![set_ignore_cursor_events])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
