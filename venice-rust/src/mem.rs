#![allow(dead_code, unused, non_snake_case)]
use crate::win::*;

pub struct Mem { pub handle: HANDLE, pub pid: u32, pub base: u64, pub size: u64 }

pub fn cstr(b: &[u8]) -> String {
    let e = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..e]).to_string()
}

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

impl Mem {
    pub fn attach(exe: &str, module: &str) -> Result<Mem, String> {
        let pid = find_pid(exe).ok_or_else(|| format!("nema procesa {}", exe))?;
        let mut h = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
        if h.is_null() {
            h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_LIMITED, 0, pid) };
        }
        if h.is_null() {
            h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_LIMITED, 0, pid) };
        }
        if h.is_null() {
            return Err(format!("OpenProcess err={}", unsafe { GetLastError() }));
        }
        let mut mb = None;
        for _ in 0..10 {
            if let Some(x) = module_base(pid, module) { mb = Some(x); break; }
            unsafe { Sleep(30); }
        }
        let (base, size) = mb.ok_or_else(|| format!("{} nije nadjen (pid {})", module, pid))?;
        Ok(Mem { handle: h, pid, base, size })
    }

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
        let b = v.to_le_bytes();
        let mut n = 0usize;
        unsafe { WriteProcessMemory(self.handle, a as *mut u8, b.as_ptr(), 4, &mut n) != 0 }
    }
    pub fn write_f32(&self, a: u64, v: f32) -> bool { self.write_u32(a, v.to_bits()) }

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
            if cstr(&pe.szExeFile).to_ascii_lowercase() == t { v.push(pe.th32ProcessID); }
            ok = Process32Next(snap, &mut pe);
        }
        CloseHandle(snap);
    }
    v
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

fn dump_mods(tag: &str, mods: &[(String, u64)]) {
    let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let path = match dir { Some(d) => d.join("mods.txt"), None => std::path::PathBuf::from("mods.txt") };
    let mut s = String::new();
    s.push_str(&format!("=== {} ===\n", tag));
    for (n, b) in mods.iter() { s.push_str(&format!("{:016X} {}\n", b, n)); }
    let _ = std::fs::write(path, s);
}

pub fn attach2(exe: &str, modules: &str) -> Result<Mem, String> {
    let cands: Vec<String> = modules.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let pids = all_pids(exe);
    if pids.is_empty() { return Err(format!("nema procesa {}", exe)); }
    let mut diag = String::new();
    for pid in pids.iter() {
        let mut h = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, *pid) };
        if h.is_null() { h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_LIMITED, 0, *pid) }; }
        if h.is_null() { h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_LIMITED, 0, *pid) }; }
        if h.is_null() {
            diag.push_str(&format!("pid{}:OpenProcess err={} ", pid, unsafe { GetLastError() }));
            continue;
        }
        let mut mods = mods_th32(*pid);
        if mods.is_empty() { mods = mods_psapi(h); }
        diag.push_str(&format!("pid{}:{}mod ", pid, mods.len()));
        for _ in 0..8 {
            for c in cands.iter() {
                if let Some(mm) = mods.iter().find(|(n, _)| n.eq_ignore_ascii_case(c)) {
                    return Ok(Mem { handle: h, pid: *pid, base: mm.1, size: 0 });
                }
            }
            unsafe { Sleep(120); }
            mods = mods_th32(*pid);
            if mods.is_empty() { mods = mods_psapi(h); }
        }
        let first: Vec<String> = mods.iter().take(7).map(|(n, _)| n.clone()).collect();
        diag.push_str(&format!("[{}] ", first.join(",")));
        dump_mods(&format!("pid {}", pid), &mods);
        unsafe { CloseHandle(h); }
    }
    Err(format!("{} ne nadjen | {} -> mods.txt", modules, diag))
}

fn read_h(h: HANDLE, a: u64, b: &mut [u8]) -> bool {
    if a == 0 { return false; }
    let mut n = 0usize;
    unsafe { ReadProcessMemory(h, a as *const u8, b.as_mut_ptr(), b.len(), &mut n) != 0 && n == b.len() }
}

fn read_u64_h(h: HANDLE, a: u64) -> u64 { let mut b = [0u8; 8]; if read_h(h, a, &mut b) { u64::from_le_bytes(b) } else { 0 } }
fn read_u16_h(h: HANDLE, a: u64) -> u16 { let mut b = [0u8; 2]; if read_h(h, a, &mut b) { u16::from_le_bytes(b) } else { 0 } }

#[repr(C)]
struct PBI { exit: i32, peb: *mut u8, affinity: usize, prio: i32, pid: usize, ppid: usize }

pub fn nt_peb(h: HANDLE) -> u64 {
    let mut p: PBI = unsafe { std::mem::zeroed() };
    let mut ret: u32 = 0;
    let st = unsafe { NtQueryInformationProcess(h, 0, &mut p as *mut PBI as *mut _, std::mem::size_of::<PBI>() as u32, &mut ret) };
    if st != 0 { return 0; }
    p.peb as u64
}

pub fn mods_peb(h: HANDLE) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    let peb = nt_peb(h);
    if peb == 0 { return v; }
    let ldr = read_u64_h(h, peb + 0x18);
    if ldr == 0 { return v; }
    let head = ldr + 0x10;
    let mut cur = read_u64_h(h, head);
    let mut n = 0;
    while cur != 0 && cur != head && n < 512 {
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

pub fn diag_errs(h: HANDLE, pid: u32) -> String {
    let mut s = String::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
        if snap == INVALID_HANDLE_VALUE { s.push_str(&format!("th32err={} ", GetLastError())); }
        else { CloseHandle(snap); s.push_str("th32ok "); }
        let mut need: DWORD = 0;
        let r = EnumProcessModulesEx(h, std::ptr::null_mut(), 0, &mut need, LIST_MODULES_ALL);
        if r == 0 { s.push_str(&format!("psapierr={} ", GetLastError())); }
        else { s.push_str(&format!("psapineed={} ", need)); }
        s.push_str(&format!("peb={:X} ", nt_peb(h)));
    }
    s
}

pub fn attach3(exe: &str, modules: &str) -> Result<Mem, String> {
    let cands: Vec<String> = modules.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let pids = all_pids(exe);
    if pids.is_empty() { return Err(format!("nema procesa {}", exe)); }
    let mut diag = String::new();
    for pid in pids.iter() {
        let mut h = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, *pid) };
        let mut rights = "all";
        if h.is_null() { h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_LIMITED, 0, *pid) }; rights = "rw+ql"; }
        if h.is_null() { h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_LIMITED, 0, *pid) }; rights = "read+ql"; }
        if h.is_null() {
            diag.push_str(&format!("pid{}:OpenProcess err={} ", pid, unsafe { GetLastError() }));
            continue;
        }
        diag.push_str(&format!("pid{}({}) ", pid, rights));
        diag.push_str(&diag_errs(h, *pid));
        let mut mods = mods_peb(h);
        if mods.is_empty() { mods = mods_th32(*pid); }
        if mods.is_empty() { mods = mods_psapi(h); }
        diag.push_str(&format!("mods={} ", mods.len()));
        for _ in 0..8 {
            for c in cands.iter() {
                if let Some(mm) = mods.iter().find(|(n, _)| n.eq_ignore_ascii_case(c)) {
                    let mut mz = [0u8; 2];
                    read_h(h, mm.1, &mut mz);
                    diag.push_str(&format!("{}@{:X} mz={:?} ", mm.0, mm.1, mz));
                    return Ok(Mem { handle: h, pid: *pid, base: mm.1, size: 0 });
                }
            }
            unsafe { Sleep(120); }
            mods = mods_peb(h);
            if mods.is_empty() { mods = mods_th32(*pid); }
            if mods.is_empty() { mods = mods_psapi(h); }
        }
        let first: Vec<String> = mods.iter().take(6).map(|(n, _)| n.clone()).collect();
        diag.push_str(&format!("[{}] ", first.join(",")));
        dump_mods(&format!("pid {}", pid), &mods);
        unsafe { CloseHandle(h); }
    }
    Err(format!("{} ne nadjen | {}", modules, diag))
}
