#![allow(dead_code, unused, non_snake_case)]
mod win; mod mem; mod cfg; mod draw; mod game; mod ui;

use win::*;
use std::ffi::c_void;

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

fn find_ini() -> String {
    if let Ok(p) = std::env::current_exe() {
        if let Some(d) = p.parent() {
            let f = d.join("rust.ini");
            if f.exists() { return f.to_string_lossy().to_string(); }
        }
    }
    "rust.ini".to_string()
}

fn main() {
    let ini = find_ini();
    let mut cfg = cfg::Cfg::load(&ini);

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

    let k_menu = vk_of(&cfg.s("key_menu", "F1"));
    let k_esp = vk_of(&cfg.s("key_esp", "F2"));
    let k_recoil = vk_of(&cfg.s("key_recoil", "F3"));
    let k_spread = vk_of(&cfg.s("key_spread", "F4"));
    let k_reload = vk_of(&cfg.s("key_reload", "F5"));
    let k_exit = vk_of(&cfg.s("key_exit", "END"));

    loop {
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        while unsafe { PeekMessageA(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe { TranslateMessage(&msg); DispatchMessageA(&msg); }
        }

        if keys.pressed(k_exit) { break; }
        if keys.pressed(k_menu) { ui.open = !ui.open; }
        if keys.pressed(k_esp) { let v = !cfg.b("esp_enabled", true); cfg.set("esp_enabled", if v { "1" } else { "0" }); }
        if keys.pressed(k_recoil) { let v = !cfg.b("norecoil", true); cfg.set("norecoil", if v { "1" } else { "0" }); }
        if keys.pressed(k_spread) { let v = !cfg.b("nospread", false); cfg.set("nospread", if v { "1" } else { "0" }); }
        if keys.pressed(k_reload) { cfg = cfg::Cfg::load(&ini); status.push("ini reload".into()); }

        for vk in 0x08..0x60 { if keys.pressed(vk) { ui.key_char(vk, &mut cfg); } }
        for vk in 0x60..0x6A { if keys.pressed(vk) { ui.key_char(vk, &mut cfg); } }

        let mut pt = POINT::default();
        unsafe { GetCursorPos(&mut pt) };
        ui.mx = pt.x;
        ui.my = pt.y;
        let lbtn = (unsafe { GetAsyncKeyState(VK_LBUTTON) } as u16 & 0x8000) != 0;
        let prev_click = ui.click;
        ui.click = lbtn && !prev_click;
        let was_click = ui.click;

        if memr.is_none() && last_attach.elapsed().as_millis() > 1000 {
            last_attach = std::time::Instant::now();
            match mem::Mem::attach(&cfg.s("exe", "RustClient.exe"), &cfg.s("module", "GameAssembly.dll")) {
                Ok(m) => {
                    status.clear();
                    status.push(format!("OK pid={} base={:X}", m.pid, m.base));
                    game = Some(game::Game::new(cfg::Cfg::load(&ini), m.base, (sw, sh)));
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
            g.cfg = cfg::Cfg::load(&ini);
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
        ui.draw(&mut canvas, &mut cfg, game.as_ref().unwrap_or(&game::Game::new(cfg::Cfg::load(&ini), 0, (sw, sh))));
        let _ = was_click;

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
