#![allow(dead_code, unused)]
use crate::cfg::Cfg;
use crate::draw::Canvas;
use crate::game::Game;

pub struct Ui {
    pub open: bool,
    pub mx: i32,
    pub my: i32,
    pub click: bool,
    pub edit: Option<String>,
    pub buf: String,
    pub color: usize,
    pub status: Vec<String>,
    pub hot: i32,
    pub dirty: bool,
    pub page: u8,
    pub want_scan: bool,
    pub want_save: bool,
}

pub const COLORS: [(u8, u8, u8); 6] = [
    (0, 255, 0), (0, 255, 255), (255, 255, 0),
    (255, 0, 255), (255, 60, 60), (255, 255, 255),
];

impl Ui {
    pub fn new() -> Ui {
        Ui { open: true, mx: 0, my: 0, click: false, edit: None, buf: String::new(), color: 0,
             status: Vec::new(), hot: -1, dirty: false, page: 0, want_scan: false, want_save: false }
    }

    fn btn(&mut self, c: &mut Canvas, x: i32, y: i32, w: i32, label: &str) -> bool {
        let hot = self.mx >= x && self.mx < x + w && self.my >= y && self.my < y + 16;
        let (r, g, b) = if hot { (0, 150, 200) } else { (40, 44, 54) };
        c.rect(x, y, w, 16, r, g, b, 240);
        c.frame(x, y, w, 16, 0, 180, 255, 255);
        c.text(x + 5, y + 5, label, 255, 255, 255, 255, 1);
        hot && self.click
    }

    fn row(&mut self, c: &mut Canvas, cfg: &mut Cfg, y: &mut i32, label: &str, key: &str, kind: u8, step: u64) {
        let x = 20;
        let w = 300;
        let h = 18;
        let hot = self.mx >= x && self.mx < x + w && self.my >= *y && self.my < *y + h;
        if hot { self.hot = *y; }
        c.rect(x, *y, w, h, 30, 30, 38, 230);
        let editing = self.edit.as_deref() == Some(key);
        let mut val = cfg.s(key, "0");
        if editing { val = self.buf.clone(); }
        let vnum = crate::cfg::num(&val);

        c.text(x + 6, *y + 5, label, 210, 210, 220, 255, 1);

        if kind == 0 {
            let on = cfg.b(key, false);
            let (r, g, b) = if on { (0, 220, 0) } else { (160, 60, 60) };
            c.text(x + w - 60, *y + 5, if on { "ON" } else { "OFF" }, r, g, b, 255, 1);
            if hot && self.click { cfg.set(key, if on { "0" } else { "1" }); self.dirty = true; }
        } else if kind == 1 {
            c.text(x + w - 130, *y + 5, "-", 255, 255, 255, 255, 1);
            c.text(x + w - 78, *y + 5, &format!("{:X}", vnum), 255, 220, 80, 255, 1);
            c.text(x + w - 24, *y + 5, "+", 255, 255, 255, 255, 1);
            if hot && self.click {
                if self.mx > x + w - 60 {
                    cfg.set(key, &format!("{:X}", vnum.saturating_add(step)));
                } else {
                    cfg.set(key, &format!("{:X}", vnum.saturating_sub(step)));
                }
                self.dirty = true;
            }
        } else {
            let shown = if editing { format!("{}_", self.buf) } else { format!("{:X}", vnum) };
            c.text(x + w - 110, *y + 5, &shown, 255, 220, 80, 255, 1);
            if hot && self.click {
                if editing { self.edit = None; }
                else { self.edit = Some(key.to_string()); self.buf = format!("{:X}", vnum); }
            }
        }
        *y += h + 2;
    }

    pub fn draw(&mut self, c: &mut Canvas, cfg: &mut Cfg, g: &Game) {
        if !self.open { return; }
        self.hot = -1;
        let x = 10;
        let mut y = 10;
        let w = 340;
        let hgt = if self.page == 0 { 268 } else { 470 };
        c.rect(x, y, w, hgt, 12, 12, 18, 235);
        c.frame(x, y, w, hgt, 0, 180, 255, 255);
        c.text(x + 8, y + 6, "VENICE RUST v3.3", 0, 220, 255, 255, 1);
        c.text(x + 160, y + 6, if self.page == 0 { "[ MAIN ]" } else { "[ OFFSETS ]" }, 255, 220, 80, 255, 1);
        let hp = self.mx >= x + 258 && self.mx < x + 330 && self.my >= y + 4 && self.my < y + 20;
        if hp && self.click { self.page = 1 - self.page; }
        y += 24;

        if self.page == 0 {
            self.row(c, cfg, &mut y, "ESP", "esp_enabled", 0, 1);
            self.row(c, cfg, &mut y, "BOX", "esp_box", 0, 1);
            self.row(c, cfg, &mut y, "IME", "esp_name", 0, 1);
            self.row(c, cfg, &mut y, "HP", "esp_health", 0, 1);
            self.row(c, cfg, &mut y, "DIST", "esp_distance", 0, 1);
            self.row(c, cfg, &mut y, "MAX DIST", "esp_max_distance", 1, 25);
            self.row(c, cfg, &mut y, "SCALE", "esp_text_scale", 1, 1);
            self.row(c, cfg, &mut y, "NORECOIL", "norecoil", 0, 1);
            self.row(c, cfg, &mut y, "NOSPREAD", "nospread", 0, 1);
            self.row(c, cfg, &mut y, "CAM AUTO", "cam_auto", 0, 1);

            let cl = COLORS[self.color];
            let hotc = self.mx >= x + 8 && self.mx < x + 100 && self.my >= y && self.my < y + 18;
            c.rect(x + 6, y, 300, 18, 30, 30, 38, 230);
            c.text(x + 10, y + 5, "BOJA", 210, 210, 220, 255, 1);
            c.rect(x + 250, y + 4, 48, 10, cl.0, cl.1, cl.2, 255);
            if hotc && self.click { self.color = (self.color + 1) % COLORS.len(); }
            y += 22;

            let s = format!("local {:X}  view {}  cam {}", g.local, if g.view_ok { "OK" } else { "--" }, g.cam_src);
            c.text(x + 8, y, &s, 120, 180, 255, 255, 1);
            y += 14;
        } else {
            if self.btn(c, x + 8, y, 90, "SCAN CAM") { self.want_scan = true; }
            if self.btn(c, x + 104, y, 90, "SAVE CAM") { self.want_save = true; }
            let cs = format!("off {:X} d {:X}", g.cam_off, g.cam_delta);
            c.text(x + 202, y + 5, &cs, 255, 220, 80, 255, 1);
            y += 22;

            self.row(c, cfg, &mut y, "REQ UID", "require_uid", 0, 1);
            self.row(c, cfg, &mut y, "HANDLE 0a1u642u323n", "handle_mode_num", 1, 1);

            c.text(x + 8, y + 4, "--- OFSETI (klik = edit hex) ---", 120, 180, 255, 255, 1);
            y += 20;
            let keys = [
                "localplayer_static", "static_fields", "localplayer_entity", "hv_in_wrapper",
                "bn_static", "bn_list_off", "list_items_off", "list_count_off", "array_data",
                "player_name", "player_uid", "player_health", "player_inventory",
                "camera_static", "camera_instance", "camera_via_player", "camera_matrix_off",
                "w2c_off", "recoil_props", "item_heldentity",
            ];
            for k in keys.iter() { self.row(c, cfg, &mut y, k, k, 2, 1); }
        }

        y += 4;
        c.text(x + 8, y, "STATUS:", 255, 180, 0, 255, 1);
        y += 14;
        for s in self.status.iter().take(6) {
            c.text(x + 8, y, s, 200, 200, 200, 255, 1);
            y += 12;
        }
        y += 2;
        c.text(x + 8, y, "F1 menu F2 esp F3 recoil F4 spread F5 reload END exit", 140, 140, 160, 255, 1);
    }

    pub fn key_char(&mut self, vk: i32, cfg: &mut Cfg) {
        if self.edit.is_none() { return; }
        let ch = match vk {
            0x30..=0x39 => Some((vk as u8) as char),
            0x41..=0x46 => Some((vk as u8) as char),
            0x60..=0x69 => Some((b'0' + (vk - 0x60) as u8) as char),
            _ => None,
        };
        if let Some(ch) = ch { if self.buf.len() < 12 { self.buf.push(ch); } }
        if vk == crate::win::VK_BACK { self.buf.pop(); }
        if vk == crate::win::VK_RETURN {
            if let Some(k) = self.edit.clone() { cfg.set(&k, &self.buf.clone()); self.dirty = true; }
            self.edit = None;
        }
        if vk == crate::win::VK_ESCAPE { self.edit = None; }
    }
}
