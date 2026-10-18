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

pub fn cstr(b: &[u8]) -> String {
    let e = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..e]).to_string()
}

pub fn exe_dir() -> std::path::PathBuf {
    std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub fn write_log(name: &str, s: &str) {
    let _ = std::fs::write(exe_dir().join(name), s);
}

fn stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0)
}

// ---------- elevacija / privilegije ----------

pub fn is_admin() -> bool {
    unsafe {
        let mut tok: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok) == 0 { return false; }
        let mut el: TOKEN_ELEVATION = std::mem::zeroed();
        let mut ret: DWORD = 0;
        let ok = GetTokenInformation(tok, TokenElevation, &mut el as *mut _ as *mut c_void,
                                     std::mem::size_of::<TOKEN_ELEVATION>() as DWORD, &mut ret);
        CloseHandle(tok);
        ok != 0 && el.TokenIsElevated != 0
    }
}

pub fn enable_debug_priv() -> bool {
    unsafe {
        let mut tok: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut tok) == 0 {
            return false;
        }
        let mut luid: LUID = std::mem::zeroed();
        if LookupPrivilegeValueA(std::ptr::null(), b"SeDebugPrivilege\0".as_ptr(), &mut luid) == 0 {
            CloseHandle(tok);
            return false;
        }
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
        sei.lpDirectory = std::ptr::null();
        sei.nShow = SW_SHOWNORMAL;
        ShellExecuteExA(&mut sei) != 0
    }
}

// ---------- procesi / moduli ----------

pub fn find_pid(name: &str) -> Option<u32> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return None; }
        let mut pe: PROCESSENTRY32 = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
        let mut ok = Process32First(snap, &mut pe);
        let t = name.to_ascii_lowercase();
        let mut r = None;
        while ok != 0 {
            if cstr(&pe.szExeFile).to_ascii_lowercase() == t { r = Some(pe.th32ProcessID); break; }
            ok = Process32Next(snap, &mut pe);
        }
        CloseHandle(snap);
        r
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
            if n == t || n.trim_end_matches(".exe") == t.trim_end_matches(".exe") {
                v.push(pe.th32ProcessID);
            }
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
            if n.to_ascii_lowercase().contains("rust") {
                v.push(format!("{} [{}]", n, pe.th32ProcessID));
            }
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
            if cstr(&me.szModule).to_ascii_lowercase() == t {
                r = Some((me.modBaseAddr as u64, me.modBaseSize as u64));
                break;
            }
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
        while ok != 0 {
            v.push((cstr(&me.szModule), me.modBaseAddr as u64));
            ok = Module32Next(snap, &mut me);
        }
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
    let st = unsafe { NtQueryInformationProcess(h, 0, &mut p as *mut PBI as *mut c_void,
                                                std::mem::size_of::<PBI>() as u32, &mut ret) };
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

pub fn mods_peb(h: HANDLE) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    let peb = nt_peb(h);
    if peb == 0 { return v; }
    let ldr = read_u64_h(h, peb + 0x18);
    if ldr == 0 { return v; }
    let head = ldr + 0x10;
    let mut cur = read_u64_h(h, head);
    let mut n = 0;
    while cur != 0 && cur != head && n < 1024 {
        let base = read_u64_h(h, cur + 0x30);
        let len = read_u16_h(h, cur + 0x58) as usize;
        let buf = read_u64_h(h, cur + 0x60);
        if buf != 0 && len > 1 && len < 520 {
            let mut raw = vec![0u8; len];
            if read_h(h, buf, &mut raw) {
                let w: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                let nm = String::from_utf16_lossy(&w);
                if !nm.is_empty() { v.push((nm, base)); }
            }
        }
        cur = read_u64_h(h, cur);
        n += 1;
    }
    v
}

// NtQueryVirtualMemory(MemoryMappedFilenameInformation) walk - radi samo s VM_READ
pub fn mod_base_vm(h: HANDLE, name: &str) -> Option<u64> {
    let mut addr: u64 = 0x10000;
    let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    let mut buf = vec![0u8; 2048];
    let mut guard = 0u32;
    while addr < 0x0000_7FFF_FFFF_0000 && guard < 400000 {
        guard += 1;
        let r = unsafe { VirtualQueryEx(h, addr as *const c_void, &mut mbi,
                                        std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) };
        if r == 0 { break; }
        let size = mbi.RegionSize as u64;
        if mbi.State == MEM_COMMIT && mbi.Type == MEM_IMAGE {
            let mut ret: SIZE_T = 0;
            let st = unsafe { NtQueryVirtualMemory(h, mbi.BaseAddress, MemoryMappedFilenameInformation,
                                                   buf.as_mut_ptr() as *mut c_void, buf.len(), &mut ret) };
            if st == 0 {
                let us = unsafe { &*(buf.as_ptr() as *const UNICODE_STRING) };
                let len = (us.Length as usize) / 2;
                if len > 0 && len < 512 && !us.Buffer.is_null() {
                    let mut w = Vec::with_capacity(len);
                    for i in 0..len { w.push(unsafe { *us.Buffer.add(i) }); }
                    let s = String::from_utf16_lossy(&w);
                    let file = s.rsplit('\\').next().unwrap_or("");
                    if file.eq_ignore_ascii_case(name) {
                        return Some(mbi.BaseAddress as u64);
                    }
                }
            }
        }
        addr += if size == 0 { 0x1000 } else { size };
    }
    None
}

// PE export-name scan - radi samo s VM_READ, ne treba snapshot
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

pub fn mod_base_pe(h: HANDLE, name: &str) -> Option<u64> {
    let mut addr: u64 = 0x10000;
    let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    let mut guard = 0u32;
    while addr < 0x0000_7FFF_FFFF_0000 && guard < 400000 {
        guard += 1;
        let r = unsafe { VirtualQueryEx(h, addr as *const c_void, &mut mbi,
                                        std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) };
        if r == 0 { break; }
        let size = mbi.RegionSize as u64;
        if mbi.State == MEM_COMMIT && mbi.Type == MEM_IMAGE {
            if let Some(n) = pe_name(h, mbi.BaseAddress as u64) {
                if n.eq_ignore_ascii_case(name) { return Some(mbi.BaseAddress as u64); }
            }
        }
        addr += if size == 0 { 0x1000 } else { size };
    }
    None
}

fn open_best(pid: u32) -> Option<(HANDLE, &'static str, bool)> {
    unsafe {
        let mut h = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if !h.is_null() { return Some((h, "all", true)); }
        h = OpenProcess(PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION, 0, pid);
        if !h.is_null() { return Some((h, "rw+qi", true)); }
        h = OpenProcess(PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_LIMITED, 0, pid);
        if !h.is_null() { return Some((h, "rw+ql", true)); }
        h = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid);
        if !h.is_null() { return Some((h, "ro+qi", false)); }
        h = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_LIMITED, 0, pid);
        if !h.is_null() { return Some((h, "ro+ql", false)); }
        None
    }
}

fn find_fast(h: HANDLE, pid: u32, name: &str) -> Option<(u64, u64, &'static str)> {
    if let Some((b, s)) = module_base(pid, name) { return Some((b, s, "toolhelp")); }
    for (n, b) in mods_psapi(h) { if n.eq_ignore_ascii_case(name) { return Some((b, 0, "psapi")); } }
    for (n, b) in mods_peb(h) { if n.eq_ignore_ascii_case(name) { return Some((b, 0, "peb")); } }
    None
}

fn find_slow(h: HANDLE, name: &str) -> Option<(u64, u64, &'static str)> {
    if let Some(b) = mod_base_vm(h, name) { return Some((b, 0, "vm-map")); }
    if let Some(b) = mod_base_pe(h, name) { return Some((b, 0, "pe-scan")); }
    None
}

pub fn attach4(exe: &str, modules: &str) -> Result<Mem, String> {
    let cands: Vec<String> = modules.split(',').map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()).collect();
    if cands.is_empty() { return Err("module= prazan u ini".to_string()); }

    let admin = is_admin();
    let dbg = enable_debug_priv();

    let mut log = String::new();
    log.push_str(&format!("=== venice attach4 t={} ===\n", stamp()));
    log.push_str(&format!("admin={} debugpriv={} bitness={}\n", admin, dbg, std::mem::size_of::<usize>() * 8));
    if !admin {
        log.push_str("UPOZORENJE: nisi admin -> RustClient je vjerovatno High integrity -> err=5\n");
    }
    if !dbg {
        log.push_str("UPOZORENJE: SeDebugPrivilege nije ukljucen\n");
    }

    let pids = all_pids(exe);
    log.push_str(&format!("pids({}) = {:?}\n", exe, pids));
    if pids.is_empty() {
        let e = format!("nema procesa {}", exe);
        write_log("attach_log.txt", &log);
        return Err(e);
    }

    let mut diag = String::new();

    for pid in pids.iter() {
        let (h, rights, writable) = match open_best(*pid) {
            Some(x) => x,
            None => {
                diag.push_str(&format!("pid{}:OpenProcess err={} ", pid, unsafe { GetLastError() }));
                continue;
            }
        };

        let mut img = [0u8; 512];
        let mut ilen: DWORD = 512;
        let imgs = if unsafe { QueryFullProcessImageNameA(h, 0, img.as_mut_ptr(), &mut ilen) } != 0 {
            String::from_utf8_lossy(&img[..ilen as usize]).to_string()
        } else { "?".to_string() };

        diag.push_str(&format!("pid{}[{}]({}) ", pid, rights, imgs));

        let mut got: Option<(String, u64, u64, String)> = None;

        for attempt in 0..12u32 {
            for c in cands.iter() {
                if let Some((b, sz, how)) = find_fast(h, *pid, c) {
                    got = Some((c.clone(), b, sz, format!("{}", how)));
                    break;
                }
            }
            if got.is_some() { break; }
            if attempt >= 1 {
                for c in cands.iter() {
                    if let Some((b, sz, how)) = find_slow(h, c) {
                        got = Some((c.clone(), b, sz, format!("{}", how)));
                        break;
                    }
                }
                if got.is_some() { break; }
            }
            unsafe { Sleep(200); }
        }

        match got {
            Some((name, base, size, how)) => {
                let mut mz = [0u8; 2];
                let mz_ok = read_h(h, base, &mut mz) && mz[0] == b'M' && mz[1] == b'Z';
                log.push_str(&format!("OK pid={} module={} base={:X} size={} via={} mz={}\n",
                                      pid, name, base, size, how, mz_ok));
                if !mz_ok {
                    log.push_str("  (MZ nije procitan - base je vjerovatno pogresan)\n");
                }
                write_log("attach_log.txt", &log);
                return Ok(Mem {
                    handle: h, pid: *pid, base, size, writable,
                    found_via: how, base_name: name, elevated: admin,
                    cache: RefCell::new(HashMap::new()),
                });
            }
            None => {
                let first: Vec<String> = mods_peb(h).iter().take(6).map(|(n, _)| n.clone()).collect();
                let t32 = mods_th32(*pid);
                let ps = mods_psapi(h);
                diag.push_str(&format!("peb={:X} th32={} psapi={} peb={} [{}] ",
                                       nt_peb(h), t32.len(), ps.len(),
                                       mods_peb(h).len(), first.join(",")));
                let mut all = String::new();
                for (n, b) in t32.iter().chain(ps.iter()) {
                    all.push_str(&format!("{:016X} {}\n", b, n));
                }
                write_log("mods.txt", &all);
                unsafe { CloseHandle(h); }
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

    // pise samo ako se vrijednost promijenila -> manje WPM poziva
    pub fn write_f32_cached(&self, a: u64, v: f32) {
        if !self.writable || a == 0 { return; }
        {
            let c = self.cache.borrow();
            if let Some(old) = c.get(&a) {
                if (*old - v).abs() < 1e-6 { return; }
            }
        }
        if self.write_f32(a, v) {
            self.cache.borrow_mut().insert(a, v);
        } else {
            self.cache.borrow_mut().remove(&a);
        }
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

