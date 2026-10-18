#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, unused)]
use std::ffi::c_void;

pub type HWND = *mut c_void;
pub type HANDLE = *mut c_void;
pub type HDC = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HGDIOBJ = *mut c_void;
pub type HMODULE = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HKEY = *mut c_void;
pub type LPARAM = isize;
pub type WPARAM = usize;
pub type LRESULT = isize;
pub type BOOL = i32;
pub type DWORD = u32;
pub type UINT = u32;
pub type LONG = i32;
pub type WORD = u16;
pub type ATOM = u16;
pub type SIZE_T = usize;
pub type NTSTATUS = i32;

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
pub const HWND_TOPMOST: HWND = -1isize as HWND;

pub const PROCESS_ALL_ACCESS: DWORD = 0x001F0FFF;
pub const PROCESS_VM_READ: DWORD = 0x0010;
pub const PROCESS_VM_WRITE: DWORD = 0x0020;
pub const PROCESS_VM_OPERATION: DWORD = 0x0008;
pub const PROCESS_QUERY_INFORMATION: DWORD = 0x0400;
pub const PROCESS_QUERY_LIMITED: DWORD = 0x1000;
pub const PROCESS_DUP_HANDLE: DWORD = 0x0040;
pub const DUPLICATE_SAME_ACCESS: DWORD = 0x00000002;
pub const STILL_ACTIVE: DWORD = 259;

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
pub const SW_SHOWNORMAL: i32 = 1;
pub const SWP_NOACTIVATE: UINT = 0x0010;

pub const PM_REMOVE: UINT = 0x0001;
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const DIB_RGB_COLORS: UINT = 0;
pub const BI_RGB: DWORD = 0;

pub const VK_LBUTTON: i32 = 0x01;
pub const VK_RETURN: i32 = 0x0D;
pub const VK_ESCAPE: i32 = 0x1B;
pub const VK_BACK: i32 = 0x08;

pub const TOKEN_ADJUST_PRIVILEGES: DWORD = 0x0020;
pub const TOKEN_QUERY: DWORD = 0x0008;
pub const SE_PRIVILEGE_ENABLED: DWORD = 0x00000002;
pub const TOKEN_ELEVATION: u32 = 20;
pub const TOKEN_INTEGRITY_LEVEL: u32 = 25;

pub const SEE_MASK_NOCLOSEPROCESS: DWORD = 0x00000040;

pub const MEM_COMMIT: DWORD = 0x1000;
pub const MEM_IMAGE: DWORD = 0x1000000;
pub const MEMORY_MAPPED_FILENAME_INFORMATION: u32 = 2;

pub const LIST_MODULES_ALL: DWORD = 0x03;

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

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct MODULEINFO { pub lpBaseOfDll: *mut c_void, pub SizeOfImage: DWORD, pub EntryPoint: *mut c_void }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct LUID { pub LowPart: u32, pub HighPart: i32 }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct LUID_AND_ATTRIBUTES { pub Luid: LUID, pub Attributes: DWORD }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct TOKEN_PRIVILEGES { pub PrivilegeCount: DWORD, pub Privileges: [LUID_AND_ATTRIBUTES; 1] }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct TOKEN_ELEVATION_STRUCT { pub TokenIsElevated: DWORD }

#[repr(C)]
pub struct SHELLEXECUTEINFOA {
    pub cbSize: DWORD,
    pub fMask: u32,
    pub hwnd: HWND,
    pub lpVerb: *const u8,
    pub lpFile: *const u8,
    pub lpParameters: *const u8,
    pub lpDirectory: *const u8,
    pub nShow: i32,
    pub hInstApp: HINSTANCE,
    pub lpIDList: *mut c_void,
    pub lpClass: *const u8,
    pub hkeyClass: HKEY,
    pub dwHotKey: DWORD,
    pub hIcon: HANDLE,
    pub hProcess: HANDLE,
}

#[repr(C)]
pub struct MEMORY_BASIC_INFORMATION {
    pub BaseAddress: *mut c_void,
    pub AllocationBase: *mut c_void,
    pub AllocationProtect: DWORD,
    pub PartitionId: WORD,
    pub RegionSize: SIZE_T,
    pub State: DWORD,
    pub Protect: DWORD,
    pub Type: DWORD,
}

#[repr(C)]
pub struct UNICODE_STRING { pub Length: u16, pub MaximumLength: u16, pub Buffer: *mut u16 }

#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct SYSTEM_HANDLE_TABLE_ENTRY_INFO_EX {
    pub Object: *mut c_void,
    pub UniqueProcessId: usize,
    pub HandleValue: usize,
    pub GrantedAccess: u32,
    pub CreatorBackTraceIndex: u16,
    pub ObjectTypeIndex: u16,
    pub HandleAttributes: u32,
    pub Reserved: u32,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn OpenProcess(a: DWORD, b: BOOL, c: DWORD) -> HANDLE;
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn ReadProcessMemory(h: HANDLE, base: *const u8, buf: *mut u8, size: usize, read: *mut usize) -> BOOL;
    pub fn WriteProcessMemory(h: HANDLE, base: *mut u8, buf: *const u8, size: usize, written: *mut usize) -> BOOL;
    pub fn GetLastError() -> DWORD;
    pub fn SetLastError(code: DWORD);
    pub fn Sleep(ms: DWORD);
    pub fn GetModuleHandleA(name: *const u8) -> HMODULE;
    pub fn CreateToolhelp32Snapshot(flags: DWORD, pid: DWORD) -> HANDLE;
    pub fn Process32First(h: HANDLE, pe: *mut PROCESSENTRY32) -> BOOL;
    pub fn Process32Next(h: HANDLE, pe: *mut PROCESSENTRY32) -> BOOL;
    pub fn Module32First(h: HANDLE, me: *mut MODULEENTRY32) -> BOOL;
    pub fn Module32Next(h: HANDLE, me: *mut MODULEENTRY32) -> BOOL;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn GetCurrentProcessId() -> DWORD;
    pub fn GetModuleFileNameA(h: HMODULE, buf: *mut u8, n: DWORD) -> DWORD;
    pub fn GetCommandLineA() -> *const u8;
    pub fn VirtualQueryEx(h: HANDLE, addr: *const c_void, mbi: *mut MEMORY_BASIC_INFORMATION, len: SIZE_T) -> SIZE_T;
    pub fn QueryFullProcessImageNameA(h: HANDLE, flags: DWORD, buf: *mut u8, n: *mut DWORD) -> BOOL;
    pub fn GetExitCodeProcess(h: HANDLE, code: *mut DWORD) -> BOOL;
    pub fn DuplicateHandle(src: HANDLE, s: HANDLE, dst: HANDLE, t: *mut HANDLE, acc: DWORD, inherit: BOOL, opts: DWORD) -> BOOL;
    pub fn IsWow64Process(h: HANDLE, wow: *mut BOOL) -> BOOL;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn OpenProcessToken(p: HANDLE, acc: DWORD, out: *mut HANDLE) -> BOOL;
    pub fn LookupPrivilegeValueA(sys: *const u8, name: *const u8, luid: *mut LUID) -> BOOL;
    pub fn AdjustTokenPrivileges(tok: HANDLE, dis: BOOL, np: *mut TOKEN_PRIVILEGES, len: DWORD,
                                 prev: *mut TOKEN_PRIVILEGES, ret: *mut DWORD) -> BOOL;
    pub fn GetTokenInformation(tok: HANDLE, class: u32, info: *mut c_void, len: DWORD, ret: *mut DWORD) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn ShellExecuteExA(sei: *mut SHELLEXECUTEINFOA) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassA(wc: *const WNDCLASSA) -> ATOM;
    pub fn CreateWindowExA(ex: DWORD, class: *const u8, name: *const u8, style: DWORD, x: i32, y: i32,
                           w: i32, h: i32, parent: HWND, menu: HANDLE, inst: HINSTANCE, param: *mut c_void) -> HWND;
    pub fn DefWindowProcA(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> LRESULT;
    pub fn PeekMessageA(m: *mut MSG, h: HWND, min: UINT, max: UINT, remove: UINT) -> BOOL;
    pub fn TranslateMessage(m: *const MSG) -> BOOL;
    pub fn DispatchMessageA(m: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(code: i32);
    pub fn GetCursorPos(p: *mut POINT) -> BOOL;
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn GetSystemMetrics(idx: i32) -> i32;
    pub fn ShowWindow(h: HWND, cmd: i32) -> BOOL;
    pub fn SetWindowPos(h: HWND, after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: UINT) -> BOOL;
    pub fn UpdateLayeredWindow(h: HWND, dst: HDC, dstPt: *const POINT, size: *const SIZE, src: HDC,
                               srcPt: *const POINT, color: u32, blend: *const BLENDFUNCTION, flags: DWORD) -> BOOL;
    pub fn GetDC(h: HWND) -> HDC;
    pub fn ReleaseDC(h: HWND, dc: HDC) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn CreateDIBSection(dc: HDC, bi: *const BITMAPINFO, usage: UINT, bits: *mut *mut c_void,
                            sec: HANDLE, off: DWORD) -> HBITMAP;
    pub fn SelectObject(dc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
}

#[link(name = "psapi")]
extern "system" {
    pub fn EnumProcessModulesEx(h: HANDLE, mods: *mut HMODULE, cb: DWORD, needed: *mut DWORD, filter: DWORD) -> BOOL;
    pub fn GetModuleBaseNameA(h: HANDLE, m: HMODULE, buf: *mut u8, size: DWORD) -> DWORD;
    pub fn GetModuleInformation(h: HANDLE, m: HMODULE, mi: *mut MODULEINFO, cb: DWORD) -> BOOL;
    pub fn GetMappedFileNameA(h: HANDLE, addr: *mut c_void, buf: *mut u8, n: DWORD) -> DWORD;
}

#[link(name = "ntdll")]
extern "system" {
    pub fn NtQueryInformationProcess(h: HANDLE, class: u32, info: *mut c_void, len: u32, ret: *mut u32) -> NTSTATUS;
    pub fn NtQueryVirtualMemory(h: HANDLE, addr: *mut c_void, class: u32, info: *mut c_void,
                                len: SIZE_T, ret: *mut SIZE_T) -> NTSTATUS;
    pub fn NtReadVirtualMemory(h: HANDLE, base: *mut c_void, buf: *mut c_void, size: SIZE_T, read: *mut SIZE_T) -> NTSTATUS;
    pub fn NtWriteVirtualMemory(h: HANDLE, base: *mut c_void, buf: *const c_void, size: SIZE_T, written: *mut SIZE_T) -> NTSTATUS;
    pub fn NtQuerySystemInformation(class: u32, info: *mut c_void, len: u32, ret: *mut u32) -> NTSTATUS;
    pub fn NtQueryObject(h: HANDLE, class: u32, info: *mut c_void, len: u32, ret: *mut u32) -> NTSTATUS;
}