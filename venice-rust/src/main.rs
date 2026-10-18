#![allow(dead_code, unused, non_snake_case)]
mod win; mod mem; mod cfg; mod draw; mod game; mod ui;

use win::*;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

const DEFAULT_INI: &str = include_str!("../rust.ini");

fn vk_of(name: &str) -> i32 {
    let n = name.trim().to_ascii_uppercase();
    if n.len() >= 2 && n.starts_with('F') {
        if let Ok(k) = n[1..].parse::<i32>() {
            if k >= 1 && k <= 12 { return 0x6F + k; }
        }
    }
    match n.as_str() {
        "END" => 0x23, "INSERT" | "INS" => 0x2D, "DELETE" | "DEL" => 0x2E,
        "HOME" => 0x24, "PGUP" => 0x21, "PGDN" => 0x22,
        "ESC" | "ESCAPE" => 0x1B, "SPACE" => 0x20, "TAB" => 0x09,
        "UP" => 0x26, "DOWN" => 0x28, "LEFT" => 0x25, "RIGHT" => 0x27,
        s if s.len() == 1 => s.as_bytes()[0] as i32,
        _ => 0x70,
    }
}

struct Keys { prev: [bool; 256] }
impl Keys {
    fn new() -> Keys { Keys { prev: [false; 256] } }
    fn pressed(&mut self, vk: i32) -> bool {
        if vk < 0 || vk > 255 { return false; }
        let d = (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0;
        let was = self.prev[vk as usize];
        self.prev[vk as usize] = d;
        d && !was
    }
}

unsafe extern "system" fn wndproc(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        0x0002 => { PostQuitMessage(0); 0 }
        0x0014 => 1,
        _ => DefWindowProcA(h, m, w, l),
    }
}

fn cmdline() -> String {
    unsafe {
        let p = GetCommandLineA();
        if p.is_null() { return String::new(); }
        let mut v = Vec::new();
        for i in 0..4096isize {
            let b = *p.offset(i);
            if b == 0 { break; }
            v.push(b);
        }
        String::from_utf8_lossy(&v).to_ascii_lowercase()
    }
}

fn has_arg(a: &str) -> bool { cmdline().contains(&a.to_ascii_lowercase()) }

fn ini_paths() -> (PathBuf, PathBuf) {
    let primary = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.join("rust.ini")))
        .unwrap_or_else(|| PathBuf::from("rust.ini"));
    let mirror = std::env::var("LOCALAPPDATA")
        .map(|l| PathBuf::from(l).join("venice_rust").join("rust.ini"))
        .unwrap_or_else(|_| PathBuf::from("rust_mirror.ini"));
    (primary, mirror)
}

fn dup_cfg(c: &cfg::Cfg) -> cfg::Cfg {
    let mut n = cfg::Cfg::load("__venice_none__");
    for (k, v) in c.p.iter() { n.set(k, v); }
    n
}

fn read_cfg(primary: &Path, mirror: &Path) -> cfg::Cfg {
    if primary.exists() { return cfg::Cfg::load(&primary.to_string_lossy()); }
    if mirror.exists() { return cfg::Cfg::load(&mirror.to_string_lossy()); }
    let mut c = cfg::Cfg::load("__none__");
    for line in DEFAULT_INI.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('[') { continue; }
        if let Some(i) = line.find('=') { c.set(&line[..i].trim(), line[i + 1..].trim()); }
    }
    c
}

fn save_cfg(c: &cfg::Cfg, primary: &Path, mirror: &Path) {
    if let Some(d) = mirror.parent() { let _ = std::fs::create_dir_all(d); }
    c.save(&mirror.to_string_lossy());
    if primary.exists() { c.save(&primary.to_string_lossy()); }
}

fn main() {
    let cleanup = has_arg("--cleanup");
    let noadmin = has_arg("--noadmin");

    if !mem::is_admin() && !noadmin {
        if mem::relaunch_elevated() { return; }
    }
    let _dbg = mem::enable_debug_priv();

    let (ini, mirror) = ini_paths();
    let mut cfg = read_cfg(&ini, &mirror);
    if cfg.p.is_empty() {
        let _ = std::fs::write(&mirror, DEFAULT_INI);
        cfg = cfg::Cfg::load(&mirror.to_string_lossy());
    }

    let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    if sw < 100 || sh < 100 { return; }

    let (hwnd, mem_dc, screen_dc, bits);
    unsafe {
        let inst = GetModuleHandleA(std::ptr::null());
        let cls = b"VeniceRustOverlay\0";
        let mut wc: WNDCLASSA = std::mem::zeroed();
        wc.lpfnWndProc = Some(wndproc);
        wc.hInstance = inst;
        wc.lpszClassName = cls.as_ptr();
        RegisterClassA(&wc);
        hwnd = CreateWindowExA(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
            cls.as_ptr(), b"VENICE RUST\0".as_ptr(), WS_POPUP,
            0, 0, sw, sh, std::ptr::null_mut(), std::ptr::null_mut(), inst, std::ptr::null_mut());
        if hwnd.is_null() { return; }
        ShowWindow(hwnd, SW_SHOW);
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, sw, sh, SWP_NOACTIVATE);

        screen_dc = GetDC(std::ptr::null_mut());
        mem_dc = CreateCompatibleDC(screen_dc);
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = sw;
        bi.bmiHeader.biHeight = -sh;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;
        let mut b: *mut c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(mem_dc, &bi, DIB_RGB_COLORS, &mut b, std::ptr::null_mut(), 0);
        SelectObject(mem_dc, dib);
        bits = b as *mut u32;
    }

    let mut canvas = draw::Canvas::new(sw, sh);
    let mut ui = ui::Ui::new();
    let mut keys = Keys::new();
    let mut memr: Option<mem::Mem> = None;
    let mut game: Option<game::Game> = None;
    let mut status: Vec<String> = Vec::new();
    let mut last_attach = std::time::Instant::now();
    let mut fps_t = std::time::Instant::now();
    let mut frames = 0u32;
    let mut fps = 0f32;
    let dummy = game::Game::new(cfg::Cfg::load("__none__"), 0, (sw, sh));

    let k_menu = vk_of(&cfg.s("key_menu", "F1"));
    let k_esp = vk_of(&cfg.s("key_esp", "F2"));
    let k_recoil = vk_of(&cfg.s("key_recoil", "F3"));
    let k_spread = vk_of(&cfg.s("key_spread", "F4"));
    let k_reload = vk_of(&cfg.s("key_reload", "F5"));
    let k_exit = vk_of(&cfg.s("key_exit", "END"));

    loop {
        let mut dirty = false;
        frames += 1;
        if fps_t.elapsed().as_millis() > 500 {
            fps = frames as f32 * 1000.0 / fps_t.elapsed().as_millis() as f32;
            frames = 0;
            fps_t = std::time::Instant::now();
        }

        let mut msg: MSG = unsafe { std::mem::zeroed() };
        while unsafe { PeekMessageA(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe { TranslateMessage(&msg); DispatchMessageA(&msg); }
        }

        if keys.pressed(k_exit) { break; }
        if keys.pressed(k_menu) { ui.open = !ui.open; }
        if keys.pressed(k_esp) { let v = !cfg.b("esp_enabled", true); cfg.set("esp_enabled", if v { "1" } else { "0" }); dirty = true; }
        if keys.pressed(k_recoil) { let v = !cfg.b("norecoil", true); cfg.set("norecoil", if v { "1" } else { "0" }); dirty = true; }
        if keys.pressed(k_spread) { let v = !cfg.b("nospread", false); cfg.set("nospread", if v { "1" } else { "0" }); dirty = true; }
        if keys.pressed(k_reload) { cfg = read_cfg(&ini, &mirror); status.push("ini reload".into()); }

        for vk in 0x08..0x60 { if keys.pressed(vk) { ui.key_char(vk, &mut cfg); } }
        for vk in 0x60..0x6A { if keys.pressed(vk) { ui.key_char(vk, &mut cfg); } }

        let mut pt = POINT::default();
        unsafe { GetCursorPos(&mut pt) };
        ui.mx = pt.x;
        ui.my = pt.y;
        let lbtn = (unsafe { GetAsyncKeyState(VK_LBUTTON) } as u16 & 0x8000) != 0;
        let prev_click = ui.click;
        ui.click = lbtn && !prev_click;

        if memr.is_none() && last_attach.elapsed().as_millis() > 1200 {
            last_attach = std::time::Instant::now();
            match mem::attach4(&cfg.s("exe", "RustClient.exe"), &cfg.s("module", "GameAssembly.dll")) {
                Ok(m) => {
                    status.clear();
                    status.push(format!("OK pid={} base={:X} via={}", m.pid, m.base, m.found_via));
                    game = Some(game::Game::new(dup_cfg(&cfg), m.base, (sw, sh)));
                    memr = Some(m);
                }
                Err(e) => {
                    status.clear();
                    status.push(e);
                    for r in mem::list_rust().iter().take(4) { status.push(r.clone()); }
                }
            }
        }

        if let (Some(m), Some(g)) = (memr.as_ref(), game.as_mut()) {
            g.cfg = dup_cfg(&cfg);
            g.refresh(m);
            if cfg.b("norecoil", true) { g.norecoil(m); }
            if cfg.b("nospread", false) { g.nospread(m); }
        }

        if ui.want_scan {
            ui.want_scan = false;
            if let Some(g) = game.as_mut() { g.scan_budget = 3; g.cam_ptr = 0; }
            status.push("scan cam...".into());
        }
        if ui.want_save {
            ui.want_save = false;
            if let Some(g) = game.as_ref() {
                if g.cam_off != 0 {
                    cfg.set("camera_via_player", &format!("0x{:X}", g.cam_off));
                    cfg.set("camera_matrix_off", &format!("0x{:X}", g.cam_delta));
                    dirty = true;
                    status.push(format!("cam ofs sacuvan {:X}+{:X}", g.cam_off, g.cam_delta));
                } else {
                    status.push("cam nije nadjen".into());
                }
            }
        }

        canvas.clear();

        if let Some(g) = game.as_ref() {
            if cfg.b("esp_enabled", true) {
                let sc = cfg.u("esp_text_scale", 1).max(1) as i32;
                let maxd = cfg.u("esp_max_distance", 300) as f32;
                let col = ui::COLORS[ui.color];
                for p in g.players.iter() {
                    if p.dist > maxd || p.dist < 0.1 { continue; }
                    let (sx, sy) = match p.screen { Some(s) => s, None => continue };
                    let (hx, hy) = match p.head { Some(s) => s, None => continue };
                    let hgt = (sy - hy).abs().max(12);
                    let wid = hgt / 2;
                    let bx = sx - wid / 2;
                    let by = hy.min(sy);
                    if cfg.b("esp_box", true) {
                        canvas.frame(bx, by, wid, hgt, col.0, col.1, col.2, 230);
                        canvas.frame(bx - 1, by - 1, wid + 2, hgt + 2, 0, 0, 0, 140);
                    }
                    let mut ty = by - 10;
                    if cfg.b("esp_name", true) {
                        let nm = if p.name.is_empty() { "?" } else { p.name.as_str() };
                        canvas.text(sx - canvas.tw(nm, sc) / 2, ty, nm, 255, 255, 255, 255, sc);
                        ty -= canvas.th(sc) + 2;
                    }
                    if cfg.b("esp_distance", true) {
                        let ds = format!("{}m", p.dist as i32);
                        canvas.text(sx - canvas.tw(&ds, sc) / 2, ty, &ds, 255, 255, 0, 255, sc);
                    }
                    if cfg.b("esp_health", true) && p.hp > 0.0 {
                        let hs = format!("{}", p.hp as i32);
                        canvas.text(bx + wid + 3, by, &hs, 0, 255, 0, 255, sc);
                        let hb = (p.hp.clamp(0.0, 100.0) / 100.0 * hgt as f32) as i32;
                        canvas.rect(bx - 6, by, 3, hgt, 60, 0, 0, 200);
                        canvas.rect(bx - 6, by + (hgt - hb), 3, hb, 0, 220, 0, 220);
                    }
                }
            }
        }

        status.truncate(8);
        ui.status = status.clone();
        ui.draw(&mut canvas, &mut cfg, game.as_ref().unwrap_or(&dummy));

        if let Some(g) = game.as_ref() {
            let hud = format!("fps {:.0}  players {}  base {:X}  cam {}",
                              fps, g.players.len(), g.base, if g.view_ok { "OK" } else { "--" });
            canvas.text(12, 4, &hud, 0, 220, 255, 255, 1);
        }

        if dirty || ui.dirty {
            save_cfg(&cfg, &ini, &mirror);
            dirty = false;
            ui.dirty = false;
        }

        unsafe {
            let dst = std::slice::from_raw_parts_mut(bits, (sw * sh) as usize);
            dst.copy_from_slice(&canvas.px);
            let p0 = POINT { x: 0, y: 0 };
            let sz = SIZE { cx: sw, cy: sh };
            let bf = BLENDFUNCTION { BlendOp: AC_SRC_OVER, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA };
            UpdateLayeredWindow(hwnd, screen_dc, &p0, &sz, mem_dc, &p0, 0, &bf, ULW_ALPHA);
        }

        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    if cleanup {
        let _ = std::fs::remove_file(mem::exe_dir().join("attach_log.txt"));
        let _ = std::fs::remove_file(mem::exe_dir().join("mods.txt"));
    }
}

