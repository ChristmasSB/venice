#![allow(dead_code, unused, non_snake_case)]
use crate::cfg::{self, Cfg};
use crate::mem::Mem;

#[derive(Clone, Default)]
pub struct Player {
    pub ptr: u64,
    pub name: String,
    pub hp: f32,
    pub dist: f32,
    pub pos: [f32; 3],
    pub screen: Option<(i32, i32)>,
    pub head: Option<(i32, i32)>,
}

pub struct Game {
    pub cfg: Cfg,
    pub base: u64,
    pub screen: (i32, i32),
    pub players: Vec<Player>,
    pub local: u64,
    pub view: [f32; 16],
    pub view_ok: bool,
    pub cam_ptr: u64,
    pub cam_off: u64,
    pub cam_delta: u64,
    pub cam_src: String,
    pub scan_budget: i32,
}

pub fn w2s(m: &[f32; 16], p: [f32; 3], w: i32, h: i32) -> Option<(i32, i32)> {
    let cx = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
    let cy = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
    let cw = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
    if cw <= 0.001 { return None; }
    let sx = (w as f32 / 2.0) + (cx / cw) * (w as f32 / 2.0);
    let sy = (h as f32 / 2.0) - (cy / cw) * (h as f32 / 2.0);
    if !sx.is_finite() || !sy.is_finite() { return None; }
    Some((sx as i32, sy as i32))
}

fn read_mat(mem: &Mem, a: u64) -> [f32; 16] {
    let mut m = [0f32; 16];
    for i in 0..16 { m[i] = mem.read_f32(a + (i as u64) * 4); }
    m
}

fn valid_view(m: &[f32; 16], lp: [f32; 3], lp_ok: bool) -> bool {
    for v in m.iter() { if !v.is_finite() { return false; } }
    if m[3].abs() > 0.02 || m[7].abs() > 0.02 || m[11].abs() > 0.02 { return false; }
    if (m[15] - 1.0).abs() > 0.02 { return false; }
    let r0 = m[0] * m[0] + m[4] * m[4] + m[8] * m[8];
    let r1 = m[1] * m[1] + m[5] * m[5] + m[9] * m[9];
    let r2 = m[2] * m[2] + m[6] * m[6] + m[10] * m[10];
    for r in [r0, r1, r2] { if r < 0.5 || r > 2.0 { return false; } }
    if lp_ok {
        if (m[12] - lp[0]).abs() > 30.0 { return false; }
        if (m[13] - lp[1]).abs() > 30.0 { return false; }
        if (m[14] - lp[2]).abs() > 30.0 { return false; }
    }
    true
}

impl Game {
    pub fn new(cfg: Cfg, base: u64, screen: (i32, i32)) -> Game {
        Game {
            cfg, base, screen,
            players: Vec::new(), local: 0,
            view: [0.0; 16], view_ok: false,
            cam_ptr: 0, cam_off: 0, cam_delta: 0, cam_src: String::new(),
            scan_budget: 2,
        }
    }

    pub fn handle(&self, mem: &Mem, id: u64) -> u64 {
        if id == 0 { return 0; }
        let hm = self.cfg.u("handle_mode_num", 255);
        let mode: String = if hm == 255 {
            self.cfg.s("handle_mode", "auto")
        } else {
            match hm { 0 => "auto", 1 => "u64", 2 => "u32", _ => "none" }.to_string()
        };
        if mode == "none" { return id; }
        let v1 = id & 0xFFFF_FFFF_FFFF_E000;
        let slot = ((id & 0x1FFF) >> 3) + 5;
        let typ = mem.read_u8(v1 + 32);
        let u32mode = mode == "u32" || (mode == "auto" && typ <= 1);
        if u32mode { (!mem.read_u32(v1 + 8 * slot)) as u64 } else { mem.read_u64(v1 + 8 * slot) }
    }

    fn pos_of(&self, mem: &Mem, p: u64) -> Option<[f32; 3]> {
        let chain = self.cfg.s("pos_chain", "0x348,0x10,0x38");
        let parts: Vec<&str> = chain.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
        if parts.is_empty() { return None; }
        let mut a = p;
        for (i, st) in parts.iter().enumerate() {
            if a == 0 { return None; }
            let off = cfg::num(st);
            if i + 1 == parts.len() {
                let v = mem.read_vec3(a + off);
                if v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
                    && v[0].abs() < 20000.0 && v[2].abs() < 20000.0 {
                    return Some(v);
                }
                return None;
            }
            a = mem.read_u64(a + off);
        }
        None
    }

    pub fn resolve_local(&self, mem: &Mem) -> u64 {
        if self.base == 0 { return 0; }
        let st = self.cfg.u("localplayer_static", 0);
        if st == 0 { return 0; }
        let sf = self.cfg.u("static_fields", 0xB8);
        let ent = self.cfg.u("localplayer_entity", 0x40);
        let statics = mem.read_u64(self.base + st + sf);
        if statics == 0 { return 0; }
        let raw = mem.read_u64(statics + ent);
        let hv = self.cfg.u("hv_in_wrapper", 0);
        let h = if hv != 0 { mem.read_u64(raw + hv) } else { raw };
        self.handle(mem, h)
    }

    fn read_view(&mut self, mem: &Mem, lp: [f32; 3], lp_ok: bool) -> bool {
        let w2c = self.cfg.u("w2c_off", 0x2E0);

        if self.cam_ptr != 0 {
            let m = read_mat(mem, self.cam_ptr + self.cam_delta);
            if valid_view(&m, lp, lp_ok) { self.view = m; return true; }
            self.cam_ptr = 0;
        }

        let cam = self.cfg.u("camera_static", 0);
        if cam != 0 && self.base != 0 {
            let statics = mem.read_u64(self.base + cam + self.cfg.u("static_fields", 0xB8));
            if statics != 0 {
                let inst = mem.read_u64(statics + self.cfg.u("camera_instance", 0x30));
                if inst > 0x10000 {
                    let m = read_mat(mem, inst + w2c);
                    if valid_view(&m, lp, lp_ok) {
                        self.view = m; self.cam_ptr = inst; self.cam_delta = w2c;
                        self.cam_src = "camera_static".to_string();
                        return true;
                    }
                }
            }
        }

        let via = self.cfg.u("camera_via_player", 0);
        if via != 0 && self.local != 0 {
            let p = mem.read_u64(self.local + via);
            let d = self.cfg.u("camera_matrix_off", w2c);
            if p > 0x10000 {
                let m = read_mat(mem, p + d);
                if valid_view(&m, lp, lp_ok) {
                    self.view = m; self.cam_ptr = p; self.cam_delta = d; self.cam_off = via;
                    self.cam_src = "via_player".to_string();
                    return true;
                }
            }
        }

        if self.scan_budget > 0 && self.local != 0 && self.cfg.b("cam_auto", true) {
            self.scan_budget -= 1;
            if self.cam_scan(mem, lp, lp_ok, w2c) { return true; }
        }
        false
    }

    fn cam_scan(&mut self, mem: &Mem, lp: [f32; 3], lp_ok: bool, w2c: u64) -> bool {
        let deltas: [u64; 6] = [w2c, 0x2E0, 0x2D0, 0x2F0, 0x300, 0x280];
        for off in (0..0x800u64).step_by(8) {
            let p = mem.read_u64(self.local + off);
            if p < 0x10000 || p > 0x0000_7FFF_FFFF_FFFF { continue; }
            for d in deltas.iter() {
                let m = read_mat(mem, p + d);
                if valid_view(&m, lp, lp_ok) {
                    self.view = m; self.cam_ptr = p; self.cam_delta = *d; self.cam_off = off;
                    self.cam_src = format!("scan {:X}+{:X}", off, d);
                    return true;
                }
            }
        }
        for off in (0..0x400u64).step_by(8) {
            let p = mem.read_u64(self.local + off);
            if p < 0x10000 { continue; }
            let mut d = 0u64;
            while d < 0x400 {
                let m = read_mat(mem, p + d);
                if valid_view(&m, lp, lp_ok) {
                    self.view = m; self.cam_ptr = p; self.cam_delta = d; self.cam_off = off;
                    self.cam_src = format!("deep {:X}+{:X}", off, d);
                    return true;
                }
                d += 0x10;
            }
        }
        false
    }

    pub fn refresh(&mut self, mem: &Mem) {
        self.view_ok = false;
        self.local = self.resolve_local(mem);

        let mut lpos = [0.0f32; 3];
        let mut lp_ok = false;
        if self.local != 0 {
            if let Some(p) = self.pos_of(mem, self.local) { lpos = p; lp_ok = true; }
        }

        if self.read_view(mem, lpos, lp_ok) { self.view_ok = true; }

        self.players.clear();
        if self.local == 0 { return; }

        let bn = self.cfg.u("bn_static", 0);
        if bn == 0 || self.base == 0 { return; }
        let statics = mem.read_u64(self.base + bn + self.cfg.u("static_fields", 0xB8));
        if statics == 0 { return; }
        let lp = mem.read_u64(statics + self.cfg.u("bn_list_off", 0x20));
        if lp == 0 { return; }
        let items = mem.read_u64(lp + self.cfg.u("list_items_off", 0x10));
        let n = mem.read_i32(lp + self.cfg.u("list_count_off", 0x18));
        if items == 0 || n <= 0 || n > 3000 { return; }

        let need_uid = self.cfg.b("require_uid", true);
        let data = items + self.cfg.u("array_data", 0x20);
        let mut out: Vec<Player> = Vec::new();
        for i in 0..n {
            let raw = mem.read_u64(data + 8 * i as u64);
            if raw == 0 { continue; }
            let e = self.handle(mem, raw);
            if e == 0 || e == self.local { continue; }
            if need_uid {
                let uid = mem.read_u64(e + self.cfg.u("player_uid", 0x718));
                if uid < 76561190000000000 || uid > 76561210000000000 { continue; }
            }
            let nm_ptr = mem.read_u64(e + self.cfg.u("player_name", 0x6F0));
            let name = mem.read_ustring(nm_ptr, 40);
            let hp = mem.read_f32(e + self.cfg.u("player_health", 0x2B4));
            let mut pl = Player { ptr: e, name, hp, ..Default::default() };
            if let Some(p) = self.pos_of(mem, e) {
                pl.pos = p;
                let dx = p[0] - lpos[0];
                let dy = p[1] - lpos[1];
                let dz = p[2] - lpos[2];
                pl.dist = (dx * dx + dy * dy + dz * dz).sqrt();
            }
            if self.view_ok {
                pl.screen = w2s(&self.view, pl.pos, self.screen.0, self.screen.1);
                let head = [pl.pos[0], pl.pos[1] + self.cfg.f("head_h", 1.65), pl.pos[2]];
                pl.head = w2s(&self.view, head, self.screen.0, self.screen.1);
            }
            out.push(pl);
        }
        out.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap_or(std::cmp::Ordering::Equal));
        self.players = out;
    }

    fn held_items(&self, mem: &Mem) -> Vec<u64> {
        let mut v = Vec::new();
        if self.local == 0 { return v; }
        let inv = mem.read_u64(self.local + self.cfg.u("player_inventory", 0x510));
        if inv == 0 { return v; }
        for b in 0..3u64 {
            let cont = mem.read_u64(inv + 0x20 + b * 8);
            if cont == 0 { continue; }
            let list = mem.read_u64(cont + self.cfg.u("itemcontainer_itemlist", 0x60));
            if list == 0 { continue; }
            let items = mem.read_u64(list + 0x10);
            let n = mem.read_i32(list + 0x18);
            if items == 0 || n <= 0 || n > 128 { continue; }
            let data = items + 0x20;
            for i in 0..n {
                let it = mem.read_u64(data + 8 * i as u64);
                if it == 0 { continue; }
                let h = mem.read_u64(it + self.cfg.u("item_heldentity", 0x88));
                if h != 0 { v.push(h); }
            }
        }
        v
    }

    pub fn norecoil(&self, mem: &Mem) -> bool {
        let fields = self.cfg.s("recoil_fields", "0x18,0x1C,0x20,0x24");
        let mut any = false;
        for h in self.held_items(mem) {
            let rp = mem.read_u64(h + self.cfg.u("recoil_props", 0x3A8));
            if rp < 0x10000 { continue; }
            for f in fields.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
                mem.write_f32_cached(rp + cfg::num(f), 0.0);
            }
            any = true;
        }
        any
    }

    pub fn nospread(&self, mem: &Mem) -> bool {
        let fields = self.cfg.s("spread_fields", "0x30,0x34,0x38,0x3C");
        let mut any = false;
        for h in self.held_items(mem) {
            let rp = mem.read_u64(h + self.cfg.u("recoil_props", 0x3A8));
            if rp < 0x10000 { continue; }
            for f in fields.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
                mem.write_f32_cached(rp + cfg::num(f), 0.0);
            }
            any = true;
        }
        any
    }
}
