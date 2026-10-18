#![allow(dead_code, unused, non_snake_case)]
use crate::win::*;
use std::ffi::c_void;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Mem {
    pub handle: HANDLE,
    pub pid: u32,
    pub base: u64,
    pub size: u64,
    pub writable: bool,
    pub found_via: String,
    pub base_name: String,
    pub elevated: bool,
    cache: RefCell<HashMap<u64, f32>>,
}

unsafe impl Send for Mem {}
unsafe impl Sync for Mem {}

impl Drop for Mem {
    fn drop(&mut self) {
        unsafe { if !self.handle.is_null() { CloseHandle(self.handle); self.handle = std::ptr::null_mut(); } }
    }
}

pub fn cstr(b: &[u8]) -> String {
    let e = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..e]).to_string()
}

pub fn exe_dir() -> std::path::PathBuf {
    std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub fn write_log(name: &str, s: &str) { let _ = std::fs::write(exe_dir().join(name), s); }

fn stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn is_admin() -> bool {
    unsafe {
        let mut tok: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok) == 0 { return false; }
        let mut el: TOKEN_ELEVATION_STRUCT = std::mem::zeroed();
        let mut ret: DWORD = 0;
        let ok = GetTokenInformation(tok, TOKEN_ELEVATION, &mut el as *mut _ as *mut c_void,
                                     std::mem::size_of::<TOKEN_ELEVATION_STRUCT>() as DWORD, &mut ret);
        CloseHandle(tok);
        ok != 0 && el.TokenIsElevated != 0
    }
}

pub fn enable_debug_priv() -> bool {
    unsafe {
        let mut tok: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut tok) == 0 { return false; }
        let mut luid: LUID = std::mem::zeroed();
        if LookupPrivilegeValueA(std::ptr::null(), b"SeDebugPrivilege\0".as_ptr(), &mut luid) == 0 { CloseHandle(tok); return false; }
        let mut tp: TOKEN_PRIVILEGES = std::mem::zeroed();
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = luid;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        let ok = AdjustTokenPrivileges(tok, 0, &mut tp, 0, std::ptr::null_mut(), std::ptr::null_mut());
        CloseHandle(tok);
        ok != 0 && GetLastError() == 0
    }
}

pub fn relaunch_elevated() -> bool {
    unsafe {
        let mut buf = [0u8; 512];
        let n = GetModuleFileNameA(std::ptr::null_mut(), buf.as_mut_ptr(), 512);
        if n == 0 { return false; }
        let mut sei: SHELLEXECUTEINFOA = std::mem::zeroed();
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOA>() as DWORD;
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = b"runas\0".as_ptr();
        sei.lpFile = buf.as_ptr();
        sei.nShow = SW_SHOWNORMAL;
        ShellExecuteExA(&mut sei) != 0
    }
}

#[repr(C)] #[derive(Clone, Copy, Default)]
struct SidAndAttributes { sid: *mut c_void, attributes: DWORD }

#[repr(C)] #[derive(Clone, Copy, Default)]
struct MandatoryLabel { label: SidAndAttributes }

pub fn il_of_pid(pid: u32) -> i32 {
    unsafe {
        if pid == 0 { return -1; }
        let h = OpenProcess(PROCESS_QUERY_LIMITED, 0, pid);
        if h.is_null() { return -1; }
        let mut tok: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(h, TOKEN_QUERY, &mut tok) == 0 { CloseHandle(h); return -1; }
        let mut need: DWORD = 0;
        GetTokenInformation(tok, TOKEN_INTEGRITY_LEVEL, std::ptr::null_mut(), 0, &mut need);
        let mut buf = vec![0u8; if need < 16 { 64 } else { need as usize }];
        let ok = GetTokenInformation(tok, TOKEN_INTEGRITY_LEVEL, buf.as_mut_ptr() as *mut c_void, buf.len() as DWORD, &mut need);
        let mut lvl: i32 = -1;
        if ok != 0 {
            let ml = &*(buf.as_ptr() as *const MandatoryLabel);
            let sid = ml.label.sid;
            if !sid.is_null() {
                let cnt = *((sid as *const u8).add(1)) as usize;
                if cnt > 0 { lvl = *((sid as *const u32).add(cnt + 1)) as i32; }
            }
        }
        CloseHandle(tok);
        CloseHandle(h);
        lvl
    }
}

pub fn il_name(l: i32) -> &'static str {
    match l {
        0x1000 => "LOW",
        0x2000 => "MEDIUM",
        0x3000 => "HIGH(ADMIN)",
        0x4000 => "SYSTEM",
        -1 => "n/a",
        _ => "?",
    }
}

pub fn all_pids(name: &str) -> Vec<u32> {
    let mut v = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return v; }
        let mut pe: PROCESSENTRY32 = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
        let mut ok = Process32First(snap, &mut pe);
        let t = name.to_ascii_lowercase();
        while ok != 0 {
            let n = cstr(&pe.szExeFile).to_ascii_lowercase();
            if n == t || n.trim_end_matches(".exe") == t.trim_end_matches(".exe") { v.push(pe.th32ProcessID); }
            ok = Process32Next(snap, &mut pe);
        }
        CloseHandle(snap);
    }
    v
}

pub fn all_pids_contains(name: &str) -> Vec<u32> {
    let stem = name.trim_end_matches(".exe").trim_end_matches(".EXE").to_ascii_lowercase();
    let mut v = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return v; }
        let mut pe: PROCESSENTRY32 = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
        let mut ok = Process32First(snap, &mut pe);
        while ok != 0 {
            let n = cstr(&pe.szExeFile).to_ascii_lowercase();
            if n.starts_with(&stem) && !n.contains("venice") { v.push(pe.th32ProcessID); }
            ok = Process32Next(snap, &mut pe);
        }
        CloseHandle(snap);
    }
    v
}

pub fn list_rust() -> Vec<String> {
    let mut v = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return v; }
        let mut pe: PROCESSENTRY32 = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
        let mut ok = Process32First(snap, &mut pe);
        while ok != 0 {
            let n = cstr(&pe.szExeFile);
            if n.to_ascii_lowercase().contains("rust") { v.push(format!("{} [{}]", n, pe.th32ProcessID)); }
            ok = Process32Next(snap, &mut pe);
        }
        CloseHandle(snap);
    }
    v
}

pub fn module_base(pid: u32, name: &str) -> Option<(u64, u64)> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
        if snap == INVALID_HANDLE_VALUE { return None; }
        let mut me: MODULEENTRY32 = std::mem::zeroed();
        me.dwSize = std::mem::size_of::<MODULEENTRY32>() as u32;
        let t = name.to_ascii_lowercase();
        let mut ok = Module32First(snap, &mut me);
        let mut r = None;
        while ok != 0 {
            if cstr(&me.szModule).to_ascii_lowercase() == t { r = Some((me.modBaseAddr as u64, me.modBaseSize as u64)); break; }
            ok = Module32Next(snap, &mut me);
        }
        CloseHandle(snap);
        r
    }
}

pub fn mods_th32(pid: u32) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
        if snap == INVALID_HANDLE_VALUE { return v; }
        let mut me: MODULEENTRY32 = std::mem::zeroed();
        me.dwSize = std::mem::size_of::<MODULEENTRY32>() as u32;
        let mut ok = Module32First(snap, &mut me);
        while ok != 0 { v.push((cstr(&me.szModule), me.modBaseAddr as u64)); ok = Module32Next(snap, &mut me); }
        CloseHandle(snap);
    }
    v
}

pub fn mods_psapi(h: HANDLE) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    unsafe {
        let mut need: DWORD = 0;
        if EnumProcessModulesEx(h, std::ptr::null_mut(), 0, &mut need, LIST_MODULES_ALL) == 0 { return v; }
        let n = (need as usize) / std::mem::size_of::<HMODULE>();
        if n == 0 || n > 8192 { return v; }
        let mut buf: Vec<HMODULE> = vec![std::ptr::null_mut(); n];
        if EnumProcessModulesEx(h, buf.as_mut_ptr(), need, &mut need, LIST_MODULES_ALL) == 0 { return v; }
        for m in buf.iter() {
            let mut nb = [0u8; 260];
            let l = GetModuleBaseNameA(h, *m, nb.as_mut_ptr(), 260);
            if l == 0 { continue; }
            let mut mi: MODULEINFO = std::mem::zeroed();
            GetModuleInformation(h, *m, &mut mi, std::mem::size_of::<MODULEINFO>() as u32);
            v.push((cstr(&nb[..l as usize]), mi.lpBaseOfDll as u64));
        }
    }
    v
}

#[repr(C)]
struct PBI { exit: i32, peb: *mut u8, affinity: usize, prio: i32, pid: usize, ppid: usize }

pub fn nt_peb(h: HANDLE) -> u64 {
    let mut p: PBI = unsafe { std::mem::zeroed() };
    let mut ret: u32 = 0;
    let st = unsafe { NtQueryInformationProcess(h, 0, &mut p as *mut PBI as *mut c_void, std::mem::size_of::<PBI>() as u32, &mut ret) };
    if st != 0 { return 0; }
    p.peb as u64
}

fn read_h(h: HANDLE, a: u64, b: &mut [u8]) -> bool {
    if a == 0 { return false; }
    let mut n = 0usize;
    unsafe { ReadProcessMemory(h, a as *const u8, b.as_mut_ptr(), b.len(), &mut n) != 0 && n == b.len() }
}

fn read_u64_h(h: HANDLE, a: u64) -> u64 { let mut b = [0u8; 8]; if read_h(h, a, &mut b) { u64::from_le_bytes(b) } else { 0 } }
fn read_u16_h(h: HANDLE, a: u64) -> u16 { let mut b = [0u8; 2]; if read_h(h, a, &mut b) { u16::from_le_bytes(b) } else { 0 } }

fn walk64(h: HANDLE, peb: u64) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    let ldr = read_u64_h(h, peb + 0x18);
    if ldr == 0 { return v; }
    let head = ldr + 0x10;
    let mut cur = read_u64_h(h, head);
    let mut n = 0;
    while cur > 0x10000 && cur != head && n < 512 {
        let base = read_u64_h(h, cur + 0x30);
        let len = read_u16_h(h, cur + 0x58) as usize;
        let buf = read_u64_h(h, cur + 0x60);
        if base > 0x10000 && buf != 0 && len > 1 && len < 520 {
            let mut raw = vec![0u8; len];
            if read_h(h, buf, &mut raw) {
                let w: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                v.push((String::from_utf16_lossy(&w), base));
            }
        }
        let nxt = read_u64_h(h, cur);
        if nxt == cur { break; }
        cur = nxt;
        n += 1;
    }
    v
}

fn walk32(h: HANDLE, peb: u64) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    let ldr = read_u64_h(h, peb + 0x0C) & 0xFFFF_FFFF;
    if ldr == 0 { return v; }
    let head = ldr + 0x0C;
    let mut cur = read_u64_h(h, head) & 0xFFFF_FFFF;
    let mut n = 0;
    while cur > 0x10000 && cur != head && n < 512 {
        let base = read_u64_h(h, cur + 0x18) & 0xFFFF_FFFF;
        let len = read_u16_h(h, cur + 0x2C) as usize;
        let buf = read_u64_h(h, cur + 0x30) & 0xFFFF_FFFF;
        if base > 0x10000 && buf != 0 && len > 1 && len < 520 {
            let mut raw = vec![0u8; len];
            if read_h(h, buf, &mut raw) {
                let w: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                v.push((String::from_utf16_lossy(&w), base));
            }
        }
        let nxt = read_u64_h(h, cur) & 0xFFFF_FFFF;
        if nxt == cur { break; }
        cur = nxt;
        n += 1;
    }
    v
}

pub fn mods_peb(h: HANDLE) -> Vec<(String, u64)> {
    let peb = nt_peb(h);
    if peb == 0 { return Vec::new(); }
    let mut v = walk64(h, peb);
    if v.is_empty() { v = walk32(h, peb); }
    if v.is_empty() { v = walk32(h, peb + 0x1000); }
    v
}

pub fn dump_peb(h: HANDLE) {
    let mut s = String::new();
    let peb = nt_peb(h);
    s.push_str(&format!("peb={:X}\n", peb));
    if peb != 0 {
        let mut raw = [0u8; 0x80];
        if read_h(h, peb, &mut raw) {
            for i in (0..0x80).step_by(16) {
                s.push_str(&format!("{:04X}:", i));
                for j in 0..16 { s.push_str(&format!(" {:02X}", raw[i + j])); }
                s.push('\n');
            }
        } else { s.push_str("raw RPM FAIL\n"); }
        s.push_str(&format!("ldr@18={:X} ldr@0C={:X} imgbase@10={:X} imgbase@08={:X}\n",
                            read_u64_h(h, peb + 0x18), read_u64_h(h, peb + 0x0C) & 0xFFFF_FFFF,
                            read_u64_h(h, peb + 0x10), read_u64_h(h, peb + 0x08) & 0xFFFF_FFFF));
        s.push_str(&format!("walk64={} walk32={}\n", walk64(h, peb).len(), walk32(h, peb).len()));
    }
    write_log("peb.txt", &s);
}

pub fn mod_base_gmf(h: HANDLE, name: &str) -> Option<u64> {
    let mut addr: u64 = 0x10000;
    let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    let mut buf = [0u8; 512];
    let mut guard = 0u32;
    while addr < 0x0000_7FFF_FFFF_0000 && guard < 400000 {
        guard += 1;
        if unsafe { VirtualQueryEx(h, addr as *const c_void, &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) } == 0 { break; }
        let size = mbi.RegionSize as u64;
        if mbi.State == MEM_COMMIT && mbi.Type == MEM_IMAGE {
            let n = unsafe { GetMappedFileNameA(h, mbi.BaseAddress, buf.as_mut_ptr(), buf.len() as DWORD) };
            if n > 0 {
                let f = cstr(&buf[..n as usize]);
                if f.rsplit('\\').next().unwrap_or("").eq_ignore_ascii_case(name) { return Some(mbi.BaseAddress as u64); }
            }
        }
        addr += if size == 0 { 0x1000 } else { size };
    }
    None
}

fn pe_name(h: HANDLE, base: u64) -> Option<String> {
    let mut dos = [0u8; 0x40];
    if !read_h(h, base, &mut dos) { return None; }
    if dos[0] != b'M' || dos[1] != b'Z' { return None; }
    let e_lfanew = u32::from_le_bytes([dos[0x3C], dos[0x3D], dos[0x3E], dos[0x3F]]) as u64;
    if e_lfanew < 0x40 || e_lfanew > 0x1000 { return None; }
    let mut nt = [0u8; 0x108];
    if !read_h(h, base + e_lfanew, &mut nt) { return None; }
    if &nt[0..4] != b"PE\0\0" { return None; }
    let magic = u16::from_le_bytes([nt[0x18], nt[0x19]]);
    let dd = if magic == 0x20b { 0x88usize } else { 0x78usize };
    let exp_rva = u32::from_le_bytes([nt[dd], nt[dd + 1], nt[dd + 2], nt[dd + 3]]) as u64;
    if exp_rva == 0 { return None; }
    let mut hdr = [0u8; 0x28];
    if !read_h(h, base + exp_rva, &mut hdr) { return None; }
    let name_rva = u32::from_le_bytes([hdr[0x0C], hdr[0x0D], hdr[0x0E], hdr[0x0F]]) as u64;
    if name_rva == 0 { return None; }
    let mut nb = [0u8; 128];
    if !read_h(h, base + name_rva, &mut nb) { return None; }
    let e = nb.iter().position(|&c| c == 0).unwrap_or(nb.len());
    if e == 0 { return None; }
    Some(String::from_utf8_lossy(&nb[..e]).to_string())
}

pub fn mod_base_brute(h: HANDLE, name: &str) -> Option<u64> {
    let mut buf = [0u8; 0x40];
    let bands: [(u64, u64); 2] = [
        (0x0000_7FF0_0000_0000, 0x0000_7FFF_FFFF_0000),
        (0x0000_0000_0001_0000, 0x0000_0010_0000_0000),
    ];
    for (mut a, end) in bands {
        while a < end {
            if read_h(h, a, &mut buf) && buf[0] == b'M' && buf[1] == b'Z' {
                let e = u32::from_le_bytes([buf[0x3C], buf[0x3D], buf[0x3E], buf[0x3F]]) as u64;
                if e > 0x40 && e < 0x1000 {
                    if let Some(n) = pe_name(h, a) { if n.eq_ignore_ascii_case(name) { return Some(a); } }
                }
            }
            a += 0x10000;
        }
    }
    None
}

pub fn bases_scan(h: HANDLE, out: &mut String) {
    out.push_str(&format!("=== bases scan t={} ===\n", stamp()));
    let mut buf = [0u8; 0x40];
    let bands: [(u64, u64); 2] = [
        (0x0000_0000_0001_0000, 0x0000_0010_0000_0000),
        (0x0000_7FF0_0000_0000, 0x0000_7FFF_FFFF_0000),
    ];
    let mut n = 0u32;
    for (mut a, end) in bands {
        while a < end {
            if read_h(h, a, &mut buf) && buf[0] == b'M' && buf[1] == b'Z' {
                let e = u32::from_le_bytes([buf[0x3C], buf[0x3D], buf[0x3E], buf[0x3F]]) as u64;
                if e > 0x40 && e < 0x1000 {
                    let nm = pe_name(h, a).unwrap_or_else(|| "?".to_string());
                    out.push_str(&format!("{:016X} {}\n", a, nm));
                    n += 1;
                }
            }
            a += 0x10000;
        }
    }
    out.push_str(&format!("ukupno {}\n", n));
}

fn mz(h: HANDLE, base: u64) -> bool {
    let mut b = [0u8; 2];
    read_h(h, base, &mut b) && b[0] == b'M' && b[1] == b'Z'
}

fn handle_ok(h: HANDLE, need_write: bool) -> bool {
    unsafe {
        let peb = nt_peb(h);
        if peb == 0 { return false; }
        let mut b = [0u8; 8];
        let mut n = 0usize;
        if ReadProcessMemory(h, peb as *const u8, b.as_mut_ptr(), 8, &mut n) == 0 { return false; }
        if n != 8 { return false; }
        if need_write {
            let mut w = 0usize;
            if WriteProcessMemory(h, peb as *mut u8, b.as_ptr(), 8, &mut w) == 0 { return false; }
        }
        true
    }
}

fn open_best(pid: u32) -> Option<(HANDLE, &'static str, bool)> {
    const C: [(DWORD, &str, bool); 7] = [
        (PROCESS_QUERY_LIMITED | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION, "ql+vmrw", true),
        (PROCESS_QUERY_LIMITED | PROCESS_VM_READ, "ql+vmr", false),
        (PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION, "qi+vmrw", true),
        (PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, "qi+vmr", false),
        (PROCESS_ALL_ACCESS, "all", true),
        (PROCESS_QUERY_LIMITED, "ql", false),
        (PROCESS_VM_READ, "vmr", false),
    ];
    let mut first: Option<(HANDLE, &'static str, bool)> = None;
    unsafe {
        for (r, label, w) in C {
            SetLastError(0);
            let h = OpenProcess(r, 0, pid);
            if h.is_null() { continue; }
            if handle_ok(h, w) {
                if let Some((fh, _, _)) = first { if fh != h { CloseHandle(fh); } }
                return Some((h, label, w));
            }
            if first.is_none() { first = Some((h, label, w)); } else { CloseHandle(h); }
        }
    }
    first
}

pub fn probe(pid: u32) {
    let mut s = String::new();
    s.push_str(&format!("=== venice probe pid={} t={} ===\n", pid, stamp()));
    s.push_str(&format!("admin={} selfIL={} gameIL={} bit={}\n",
                        is_admin(), il_name(il_of_pid(std::process::id())), il_name(il_of_pid(pid)),
                        std::mem::size_of::<usize>() * 8));
    const C: [(DWORD, &str); 8] = [
        (PROCESS_ALL_ACCESS, "ALL_ACCESS"),
        (PROCESS_QUERY_LIMITED, "QUERY_LIMITED"),
        (PROCESS_QUERY_INFORMATION, "QUERY_INFO"),
        (PROCESS_VM_READ, "VM_READ"),
        (PROCESS_VM_READ | PROCESS_QUERY_LIMITED, "VMR|QL"),
        (PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_LIMITED, "VM_RW|QL"),
        (PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION, "VM_RW|QI"),
        (PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION | PROCESS_QUERY_LIMITED, "VM_RW|QI|QL"),
    ];
    unsafe {
        for (r, n) in C {
            SetLastError(0);
            let h = OpenProcess(r, 0, pid);
            if h.is_null() { s.push_str(&format!("{:<12} open FAIL err={}\n", n, GetLastError())); continue; }
            let peb = nt_peb(h);
            let mut b = [0u8; 8]; let mut got = 0usize;
            let rpm = peb != 0 && ReadProcessMemory(h, peb as *const u8, b.as_mut_ptr(), 8, &mut got) != 0;
            let mut w = 0usize;
            let wpm = rpm && WriteProcessMemory(h, peb as *mut u8, b.as_ptr(), 8, &mut w) != 0;
            let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
            let vq = VirtualQueryEx(h, peb as *const c_void, &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) != 0;
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
            let snap_ok = snap != INVALID_HANDLE_VALUE;
            if snap_ok { CloseHandle(snap); }
            let mut need: DWORD = 0;
            let psapi = EnumProcessModulesEx(h, std::ptr::null_mut(), 0, &mut need, LIST_MODULES_ALL) != 0;
            s.push_str(&format!("{:<12} open OK  peb={:X} rpm={} wpm={} vq={} snap={} psapi={}\n",
                                n, peb, rpm, wpm, vq, snap_ok, psapi));
            CloseHandle(h);
        }
    }
    write_log("probe.txt", &s);
}

pub fn attach5(exe: &str, modules: &str, force_base: u64) -> Result<Mem, String> {
    let cands: Vec<String> = modules.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if cands.is_empty() { return Err("module= prazan u ini".to_string()); }

    let admin = is_admin();
    let dbg = enable_debug_priv();
    let self_il = il_of_pid(std::process::id());

    let mut log = String::new();
    log.push_str(&format!("=== venice attach5 t={} ===\n", stamp()));
    log.push_str(&format!("admin={} debugpriv={} bitness={} IL={}\n", admin, dbg, std::mem::size_of::<usize>() * 8, il_name(self_il)));

    let mut pids = all_pids(exe);
    if pids.is_empty() { pids = all_pids_contains(exe); }
    log.push_str(&format!("pids({}) = {:?}\n", exe, pids));
    if pids.is_empty() { write_log("attach_log.txt", &log); return Err(format!("nema procesa {}", exe)); }

    let mut diag = String::new();

    for pid in pids.iter() {
        let pil = il_of_pid(*pid);
        log.push_str(&format!("-- pid {} IL={}\n", pid, il_name(pil)));

        let mut first = true;
        for round in 0..4u32 {
            let (h, rights, writable) = match open_best(*pid) {
                Some(x) => x,
                None => { diag.push_str(&format!("pid{}:OpenProcess err={} ", pid, unsafe { GetLastError() })); break; }
            };

            if first {
                first = false;
                let mut img = [0u8; 512];
                let mut ilen: DWORD = 512;
                let imgs = if unsafe { QueryFullProcessImageNameA(h, 0, img.as_mut_ptr(), &mut ilen) } != 0 {
                    String::from_utf8_lossy(&img[..ilen as usize]).to_string()
                } else { "?".to_string() };
                diag.push_str(&format!("pid{}[{}]({}) ", pid, rights, imgs));
                log.push_str(&format!("pid{} rights={} writable={} img={}\n", pid, rights, writable, imgs));
            }

            if force_base != 0 && mz(h, force_base) {
                log.push_str(&format!("OK force_base pid={} base={:X}\n", pid, force_base));
                write_log("attach_log.txt", &log);
                return Ok(Mem { handle: h, pid: *pid, base: force_base, size: 0, writable,
                                found_via: "force_base".to_string(), base_name: cands[0].clone(),
                                elevated: admin, cache: RefCell::new(HashMap::new()) });
            }

            let th = mods_th32(*pid);
            let ps = mods_psapi(h);
            let pb = mods_peb(h);
            let mut got: Option<(String, u64, u64, String)> = None;

            'outer: for c in cands.iter() {
                let hit = |v: &Vec<(String, u64)>| -> Option<u64> {
                    v.iter().find(|(n, _)| n.eq_ignore_ascii_case(c)).map(|(_, b)| *b)
                };
                if let Some(b) = hit(&th) { if mz(h, b) { got = Some((c.clone(), b, 0, "toolhelp".to_string())); break 'outer; } }
                if let Some(b) = hit(&ps) { if mz(h, b) { got = Some((c.clone(), b, 0, "psapi".to_string())); break 'outer; } }
                if let Some(b) = hit(&pb) { if mz(h, b) { got = Some((c.clone(), b, 0, "peb".to_string())); break 'outer; } }
            }

            if got.is_none() && round < 2 {
                for c in cands.iter() {
                    if let Some(b) = mod_base_gmf(h, c) { if mz(h, b) { got = Some((c.clone(), b, 0, "mapped".to_string())); break; } }
                    if let Some(b) = mod_base_brute(h, c) { if mz(h, b) { got = Some((c.clone(), b, 0, "brute".to_string())); break; } }
                }
            }

            match got {
                Some((name, base, size, how)) => {
                    log.push_str(&format!("OK pid={} module={} base={:X} via={} rights={}\n", pid, name, base, how, rights));
                    write_log("attach_log.txt", &log);
                    return Ok(Mem { handle: h, pid: *pid, base, size, writable,
                                    found_via: how, base_name: name, elevated: admin,
                                    cache: RefCell::new(HashMap::new()) });
                }
                None => {
                    if round == 0 {
                        diag.push_str(&format!("th32={} psapi={} peb={} pebaddr={:X} ",
                                               th.len(), ps.len(), pb.len(), nt_peb(h)));
                        probe(*pid);
                        dump_peb(h);
                        let mut bl = String::new();
                        bases_scan(h, &mut bl);
                        write_log("bases.txt", &bl);
                    }
                    unsafe { CloseHandle(h); }
                    unsafe { Sleep(120); }
                }
            }
        }
    }

    let e = format!("{} ne nadjen | {}", modules, diag);
    log.push_str(&format!("FAIL: {}\n", e));
    write_log("attach_log.txt", &log);
    Err(e)
}

impl Mem {
    pub fn ok(&self) -> bool { !self.handle.is_null() && self.pid != 0 }
    pub fn clear_cache(&self) { self.cache.borrow_mut().clear(); }

    pub fn read(&self, a: u64, b: &mut [u8]) -> bool {
        if a == 0 { return false; }
        let mut n = 0usize;
        let ok = unsafe { ReadProcessMemory(self.handle, a as *const u8, b.as_mut_ptr(), b.len(), &mut n) };
        ok != 0 && n == b.len()
    }
    pub fn read_u8(&self, a: u64) -> u8 { let mut b = [0u8; 1]; if self.read(a, &mut b) { b[0] } else { 0 } }
    pub fn read_u32(&self, a: u64) -> u32 { let mut b = [0u8; 4]; if self.read(a, &mut b) { u32::from_le_bytes(b) } else { 0 } }
    pub fn read_i32(&self, a: u64) -> i32 { self.read_u32(a) as i32 }
    pub fn read_u64(&self, a: u64) -> u64 { let mut b = [0u8; 8]; if self.read(a, &mut b) { u64::from_le_bytes(b) } else { 0 } }
    pub fn read_f32(&self, a: u64) -> f32 { f32::from_bits(self.read_u32(a)) }
    pub fn read_vec3(&self, a: u64) -> [f32; 3] { [self.read_f32(a), self.read_f32(a + 4), self.read_f32(a + 8)] }

    pub fn write_u32(&self, a: u64, v: u32) -> bool {
        if !self.writable { return false; }
        let b = v.to_le_bytes();
        let mut n = 0usize;
        unsafe { WriteProcessMemory(self.handle, a as *mut u8, b.as_ptr(), 4, &mut n) != 0 }
    }
    pub fn write_f32(&self, a: u64, v: f32) -> bool { self.write_u32(a, v.to_bits()) }

    pub fn write_f32_cached(&self, a: u64, v: f32) {
        if !self.writable || a == 0 { return; }
        {
            let c = self.cache.borrow();
            if let Some(old) = c.get(&a) { if (*old - v).abs() < 1e-6 { return; } }
        }
        if self.write_f32(a, v) { self.cache.borrow_mut().insert(a, v); }
        else { self.cache.borrow_mut().remove(&a); }
    }
    pub fn read_ustring(&self, a: u64, max: usize) -> String {
        if a == 0 { return String::new(); }
        let len = self.read_i32(a + 0x10);
        if len <= 0 || len > max as i32 { return String::new(); }
        let mut u = vec![0u16; len as usize];
        let bytes = unsafe { std::slice::from_raw_parts_mut(u.as_mut_ptr() as *mut u8, len as usize * 2) };
        if !self.read(a + 0x14, bytes) { return String::new(); }
        String::from_utf16_lossy(&u)
    }
}

