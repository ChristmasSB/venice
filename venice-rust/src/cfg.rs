#![allow(dead_code, unused)]
use std::collections::HashMap;

pub fn num(s: &str) -> u64 {
    let t: String = s.trim().chars().filter(|c| *c != '_').collect();
    if t.is_empty() { return 0; }
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r.to_string()),
        None => (false, t),
    };
    let v = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).unwrap_or(0)
    } else if t.chars().any(|c| c.is_ascii_alphabetic()) {
        u64::from_str_radix(&t, 16).unwrap_or(0)
    } else {
        t.parse::<u64>().unwrap_or(0)
    };
    if neg { (v as i64).wrapping_neg() as u64 } else { v }
}

fn rot(v: u64, n: u32, l: bool) -> u64 {
    let n = n & 31;
    let lo = (v & 0xFFFF_FFFF) as u32;
    let hi = (v >> 32) as u32;
    let (a, b) = if l {
        (lo.rotate_left(n), hi.rotate_left(n))
    } else {
        (lo.rotate_right(n), hi.rotate_right(n))
    };
    ((b as u64) << 32) | a as u64
}

pub fn decode(v: u64, ops: &str) -> u64 {
    let mut v = v;
    for t in ops.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
        let l = t.to_ascii_lowercase();
        if let Some(r) = l.strip_prefix("rol") { v = rot(v, num(r) as u32, true); }
        else if let Some(r) = l.strip_prefix("ror") { v = rot(v, num(r) as u32, false); }
        else if let Some(r) = l.strip_prefix("xor") { v ^= num(r); }
        else if let Some(r) = l.strip_prefix("add") { v = v.wrapping_add(num(r)); }
        else if let Some(r) = l.strip_prefix("sub") { v = v.wrapping_sub(num(r)); }
    }
    v
}

pub struct Cfg { pub p: HashMap<String, String> }

impl Cfg {
    pub fn load(path: &str) -> Cfg {
        let mut p = HashMap::new();
        if let Ok(txt) = std::fs::read_to_string(path) {
            for line in txt.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with(';') || line.starts_with('#') || line.starts_with('[') { continue; }
                if let Some(i) = line.find('=') {
                    p.insert(line[..i].trim().to_ascii_lowercase(), line[i + 1..].trim().to_string());
                }
            }
        }
        Cfg { p }
    }
    pub fn s(&self, k: &str, d: &str) -> String { self.p.get(k).cloned().unwrap_or_else(|| d.to_string()) }
    pub fn u(&self, k: &str, d: u64) -> u64 { self.p.get(k).map(|v| num(v)).unwrap_or(d) }
    pub fn f(&self, k: &str, d: f32) -> f32 { self.p.get(k).map(|v| num(v) as f32).unwrap_or(d) }
    pub fn b(&self, k: &str, d: bool) -> bool { self.p.get(k).map(|v| num(v) != 0).unwrap_or(d) }
    pub fn col(&self, k: &str, d: (u8, u8, u8)) -> (u8, u8, u8) {
        match self.p.get(k) {
            Some(v) => { let n = num(v) as u32; (((n >> 16) & 0xFF) as u8, ((n >> 8) & 0xFF) as u8, (n & 0xFF) as u8) }
            None => d,
        }
    }
    pub fn set(&mut self, k: &str, v: &str) { self.p.insert(k.to_string(), v.to_string()); }
    pub fn save(&self, path: &str) {
        let mut ks: Vec<&String> = self.p.keys().collect();
        ks.sort();
        let mut out = String::new();
        out.push_str("; VENICE RUST v3.1 - auto-save\n");
        for k in ks {
            out.push_str(k);
            out.push('=');
            out.push_str(&self.p[k]);
            out.push('\n');
        }
        let _ = std::fs::write(path, out);
    }
}
