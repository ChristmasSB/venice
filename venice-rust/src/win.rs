#![allow(non_snake_case, non_camel_case_types, dead_code, unused)]
use std::ffi::c_void;

pub type HWND = *mut c_void;
pub type HANDLE = *mut c_void;
pub type HDC = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HGDIOBJ = *mut c_void;
pub type HMODULE = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type LPARAM = isize;
pub type WPARAM = usize;
pub type LRESULT = isize;
pub type BOOL = i32;
pub type DWORD = u32;
pub type UINT = u32;
pub type LONG = i32;
pub type WORD = u16;
pub type ATOM = u16;

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
pub const HWND_TOPMOST: HWND = -1isize as HWND;

pub const PROCESS_ALL_ACCESS: DWORD = 0x001F0FFF;
pub const PROCESS_VM_READ: DWORD = 0x0010;
pub const PROCESS_VM_WRITE: DWORD = 0x0020;
pub const PROCESS_VM_OPERATION: DWORD = 0x0008;
pub const PROCESS_QUERY_LIMITED: DWORD = 0x1000;

pub const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
pub const TH32CS_SNAPMODULE: DWORD = 0x00000008;
pub const TH32CS_SNAPMODULE32: DWORD = 0x00000010;

pub const WS_POPUP: DWORD = 0x80000000;
pub const WS_EX_TOPMOST: DWORD = 0x00000008;
pub const WS_EX_TRANSPARENT: DWORD = 0x00000020;
pub const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
pub const WS_EX_LAYERED: DWORD = 0x00080000;

pub const ULW_ALPHA: DWORD = 0x00000002;
pub const AC_SRC_OVER: u8 = 0x00;
pub const AC_SRC_ALPHA: u8 = 0x01;

pub const SW_SHOW: i32 = 5;
pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOMOVE: UINT = 0x0002;
pub const SWP_NOACTIVATE: UINT = 0x0010;

pub const PM_REMOVE: UINT = 0x0001;
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const DIB_RGB_COLORS: UINT = 0;
pub const BI_RGB: DWORD = 0;

pub const VK_LBUTTON: i32 = 0x01;
pub const VK_RBUTTON: i32 = 0x02;
pub const VK_RETURN: i32 = 0x0D;
pub const VK_ESCAPE: i32 = 0x1B;
pub const VK_BACK: i32 = 0x08;

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct POINT { pub x: LONG, pub y: LONG }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct SIZE { pub cx: LONG, pub cy: LONG }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct BLENDFUNCTION { pub BlendOp: u8, pub BlendFlags: u8, pub SourceConstantAlpha: u8, pub AlphaFormat: u8 }

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
    pub lPrivate: DWORD,
}

#[repr(C)]
pub struct WNDCLASSA {
    pub style: UINT,
    pub lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HANDLE,
    pub hCursor: HANDLE,
    pub hbrBackground: HANDLE,
    pub lpszMenuName: *const u8,
    pub lpszClassName: *const u8,
}

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct BITMAPINFOHEADER {
    pub biSize: DWORD,
    pub biWidth: LONG,
    pub biHeight: LONG,
    pub biPlanes: WORD,
    pub biBitCount: WORD,
    pub biCompression: DWORD,
    pub biSizeImage: DWORD,
    pub biXPelsPerMeter: LONG,
    pub biYPelsPerMeter: LONG,
    pub biClrUsed: DWORD,
    pub biClrImportant: DWORD,
}

#[repr(C)] #[derive(Clone, Copy)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [DWORD; 3],
}

#[repr(C)] #[derive(Clone, Copy)]
pub struct PROCESSENTRY32 {
    pub dwSize: DWORD,
    pub cntUsage: DWORD,
    pub th32ProcessID: DWORD,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: DWORD,
    pub cntThreads: DWORD,
    pub th32ParentProcessID: DWORD,
    pub pcPriClassBase: LONG,
    pub dwFlags: DWORD,
    pub szExeFile: [u8; 260],
}

#[repr(C)] #[derive(Clone, Copy)]
pub struct MODULEENTRY32 {
    pub dwSize: DWORD,
    pub th32ModuleID: DWORD,
    pub th32ProcessID: DWORD,
    pub GlblcntUsage: DWORD,
    pub ProccntUsage: DWORD,
    pub modBaseAddr: *mut u8,
    pub modBaseSize: DWORD,
    pub hModule: HMODULE,
    pub szModule: [u8; 256],
    pub szExePath: [u8; 260],
}

#[link(name = "kernel32")]
extern "system" {
    pub fn OpenProcess(a: DWORD, b: BOOL, c: DWORD) -> HANDLE;
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn ReadProcessMemory(h: HANDLE, base: *const u8, buf: *mut u8, size: usize, read: *mut usize) -> BOOL;
    pub fn WriteProcessMemory(h: HANDLE, base: *mut u8, buf: *const u8, size: usize, written: *mut usize) -> BOOL;
    pub fn GetLastError() -> DWORD;
    pub fn Sleep(ms: DWORD);
    pub fn GetModuleHandleA(name: *const u8) -> HMODULE;
    pub fn CreateToolhelp32Snapshot(flags: DWORD, pid: DWORD) -> HANDLE;
    pub fn Process32First(h: HANDLE, pe: *mut PROCESSENTRY32) -> BOOL;
    pub fn Process32Next(h: HANDLE, pe: *mut PROCESSENTRY32) -> BOOL;
    pub fn Module32First(h: HANDLE, me: *mut MODULEENTRY32) -> BOOL;
    pub fn Module32Next(h: HANDLE, me: *mut MODULEENTRY32) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassA(wc: *const WNDCLASSA) -> ATOM;
    pub fn CreateWindowExA(ex: DWORD, class: *const u8, name: *const u8, style: DWORD, x: i32, y: i32, w: i32, h: i32, parent: HWND, menu: HANDLE, inst: HINSTANCE, param: *mut c_void) -> HWND;
    pub fn DefWindowProcA(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> LRESULT;
    pub fn DestroyWindow(h: HWND) -> BOOL;
    pub fn PeekMessageA(m: *mut MSG, h: HWND, min: UINT, max: UINT, remove: UINT) -> BOOL;
    pub fn TranslateMessage(m: *const MSG) -> BOOL;
    pub fn DispatchMessageA(m: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(code: i32);
    pub fn GetCursorPos(p: *mut POINT) -> BOOL;
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn GetSystemMetrics(idx: i32) -> i32;
    pub fn ShowWindow(h: HWND, cmd: i32) -> BOOL;
    pub fn SetWindowPos(h: HWND, after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: UINT) -> BOOL;
    pub fn UpdateLayeredWindow(h: HWND, dst: HDC, dstPt: *const POINT, size: *const SIZE, src: HDC, srcPt: *const POINT, color: u32, blend: *const BLENDFUNCTION, flags: DWORD) -> BOOL;
    pub fn GetDC(h: HWND) -> HDC;
    pub fn ReleaseDC(h: HWND, dc: HDC) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn DeleteDC(dc: HDC) -> BOOL;
    pub fn CreateDIBSection(dc: HDC, bi: *const BITMAPINFO, usage: UINT, bits: *mut *mut c_void, sec: HANDLE, off: DWORD) -> HBITMAP;
    pub fn SelectObject(dc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(obj: HGDIOBJ) -> BOOL;
}

pub const LIST_MODULES_ALL: DWORD = 0x03;

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct MODULEINFO { pub lpBaseOfDll: *mut c_void, pub SizeOfImage: DWORD, pub EntryPoint: *mut c_void }

#[link(name = "psapi")]
extern "system" {
    pub fn EnumProcessModulesEx(h: HANDLE, mods: *mut HMODULE, cb: DWORD, needed: *mut DWORD, filter: DWORD) -> BOOL;
    pub fn GetModuleBaseNameA(h: HANDLE, m: HMODULE, buf: *mut u8, size: DWORD) -> DWORD;
    pub fn GetModuleInformation(h: HANDLE, m: HMODULE, mi: *mut MODULEINFO, cb: DWORD) -> BOOL;
}

#[link(name = "ntdll")]
extern "system" {
    pub fn NtQueryInformationProcess(h: HANDLE, class: u32, info: *mut c_void, len: u32, ret: *mut u32) -> i32;
}
