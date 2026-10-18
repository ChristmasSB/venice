#![allow(dead_code, unused, non_snake_case)]
mod win; mod mem; mod cfg; mod draw; mod game; mod ui;

use win::*;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

const DEFAULT_INI: &str = r#"; VENICE RUST v3.2 - rust.ini
; F1 menu, F2 esp, F3 norecoil, F4 nospread, F5 reload, END exit
; sve vrijednosti smiju biti hex (0x... ili golo)

[proc]
exe=RustClient.exe
module=GameAssembly.dll

[keys]
key_menu=F1
key_esp=F2
key_recoil=F3
key_spread=F4
key_reload=F5
key_exit=END

[esp]
esp_enabled=1
esp_box=1
esp_name=1
esp_health=1
esp_distance=1
esp_max_distance=300
esp_text_scale=1

[handle]
handle_mode=u64

[offsets]
localplayer_static=0x11645DB8
static_fields=0xB8
localplayer_entity=0x40
hv_in_wrapper=0x0
bn_static=0x115F0A18
bn_list_off=0x20
list_items_off=0x10
list_count_off=0x18
array_data=0x20
player_name=0x6F0
player_uid=0x718
player_health=0x2B4
player_inventory=0x510
pos_chain=0x348,0x10,0x38
head_h=1.65
camera_static=0x0
camera_instance=0x30
w2c_off=0x2E0
recoil_props=0x3A8
item_heldentity=0x88
itemcontainer_itemlist=0x60
recoil_fields=0x18,0x1C,0x20,0x24
spread_fields=0x30,0x34,0x38,0x3C
norecoil=1
nospread=0
"#;

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

extern "system" fn wndproc(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        0x0002 => { unsafe { PostQuitMessage(0) }; 0 }
        0x0014 => 1,
        _ => unsafe { DefWindowProcA(h, m, w, l) },
    }
}

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

fn read_cfg(primary: &Path, mirror: &Path, fallback: &cfg::Cfg) -> cfg::Cfg {
    if primary.exists() { return cfg::Cfg::load(&primary.to_string_lossy()); }
    if mirror.exists() { return cfg::Cfg::load(&mirror.to_string_lossy()); }
    dup_cfg(fallback)
}

fn save_cfg(c: &cfg::Cfg, primary: &Path, mirror: &Path) {
    if let Some(d) = mirror.parent() { let _ = std::fs::create_dir_all(d); }
    c.save(&mirror.to_string_lossy());
    if primary.exists() { c.save(&primary.to_string_lossy()); }
}

fn main() {
    let (ini, mirror) = ini_paths();
    let seed = cfg::Cfg::load("__venice_none__");
    if ini.exists() {
        if let Some(d) = mirror.parent() { let _ = std::fs::create_dir_all(d); }
        let _ = std::fs::copy(&ini, &mirror);
    }
    let mut cfg = read_cfg(&ini, &mirror, &seed);
    if cfg.p.is_empty() {
        if let Some(d) = mirror.parent() { let _ = std::fs::create_dir_all(d); }
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
    let dummy = game::Game::new(cfg::Cfg::load("__venice_none__"), 0, (sw, sh));

    let k_menu = vk_of(&cfg.s("key_menu", "F1"));
    let k_esp = vk_of(&cfg.s("key_esp", "F2"));
    let k_recoil = vk_of(&cfg.s("key_recoil", "F3"));
    let k_spread = vk_of(&cfg.s("key_spread", "F4"));
    let k_reload = vk_of(&cfg.s("key_reload", "F5"));
    let k_exit = vk_of(&cfg.s("key_exit", "END"));

    loop {
        let mut dirty = false;

        let mut msg: MSG = unsafe { std::mem::zeroed() };
        while unsafe { PeekMessageA(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe { TranslateMessage(&msg); DispatchMessageA(&msg); }
        }

        if keys.pressed(k_exit) { break; }
        if keys.pressed(k_menu) { ui.open = !ui.open; }
        if keys.pressed(k_esp) { let v = !cfg.b("esp_enabled", true); cfg.set("esp_enabled", if v { "1" } else { "0" }); dirty = true; }
        if keys.pressed(k_recoil) { let v = !cfg.b("norecoil", true); cfg.set("norecoil", if v { "1" } else { "0" }); dirty = true; }
        if keys.pressed(k_spread) { let v = !cfg.b("nospread", false); cfg.set("nospread", if v { "1" } else { "0" }); dirty = true; }
        if keys.pressed(k_reload) { cfg = read_cfg(&ini, &mirror, &cfg); status.push("ini reload".into()); }

        for vk in 0x08..0x60 { if keys.pressed(vk) { ui.key_char(vk, &mut cfg); } }
        for vk in 0x60..0x6A { if keys.pressed(vk) { ui.key_char(vk, &mut cfg); } }

        let mut pt = POINT::default();
        unsafe { GetCursorPos(&mut pt) };
        ui.mx = pt.x;
        ui.my = pt.y;
        let lbtn = (unsafe { GetAsyncKeyState(VK_LBUTTON) } as u16 & 0x8000) != 0;
        let prev_click = ui.click;
        ui.click = lbtn && !prev_click;

        if memr.is_none() && last_attach.elapsed().as_millis() > 1000 {
            last_attach = std::time::Instant::now();
            match mem::attach3(&cfg.s("exe", "RustClient.exe"), &cfg.s("module", "GameAssembly.dll")) {
                Ok(m) => {
                    status.clear();
                    status.push(format!("OK pid={} base={:X}", m.pid, m.base));
                    game = Some(game::Game::new(dup_cfg(&cfg), m.base, (sw, sh)));
                    memr = Some(m);
                }
                Err(e) => {
                    status.clear();
                    status.push(e);
                    for r in mem::list_rust().iter().take(6) { status.push(r.clone()); }
                }
            }
        }

        if let (Some(m), Some(g)) = (memr.as_ref(), game.as_mut()) {
            g.cfg = dup_cfg(&cfg);
            g.refresh(m);
            if cfg.b("norecoil", true) { g.norecoil(m); }
            if cfg.b("nospread", false) { g.nospread(m); }
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

        status.truncate(10);
        ui.status = status.clone();
        ui.draw(&mut canvas, &mut cfg, game.as_ref().unwrap_or(&dummy));

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
}



