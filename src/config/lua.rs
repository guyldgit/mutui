use crate::config::theme::{parse_color, Theme};
use mlua::{Lua, Table};
use ratatui::style::Modifier;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub theme: Theme,
    pub volume: f32,
    pub keys: HashMap<String, String>,
    pub leader: String,
    pub leader_keys: HashMap<String, String>,
    pub picker_keys: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut keys = HashMap::new();
        keys.insert("p".into(), "toggle_pause".into());
        keys.insert("q".into(), "quit".into());
        keys.insert("+".into(), "volume_up".into());
        keys.insert("=".into(), "volume_up".into());
        keys.insert("-".into(), "volume_down".into());
        keys.insert("r".into(), "restart".into());
        keys.insert("j".into(), "sel_down".into());
        keys.insert("k".into(), "sel_up".into());
        keys.insert("enter".into(), "play_selected".into());
        keys.insert("a".into(), "queue_add".into());
        keys.insert("d".into(), "queue_remove".into());
        keys.insert("tab".into(), "focus_next".into());
        keys.insert("n".into(), "next".into());

        let mut picker_keys = HashMap::new();
        picker_keys.insert("c-j".into(), "picker_down".into());
        picker_keys.insert("c-k".into(), "picker_up".into());
        picker_keys.insert("down".into(), "picker_down".into());
        picker_keys.insert("up".into(), "picker_up".into());
        picker_keys.insert("enter".into(), "picker_confirm".into());
        picker_keys.insert("esc".into(), "picker_close".into());
        picker_keys.insert("backspace".into(), "picker_backspace".into());

        Self {
            theme: Theme::default_dark(),
            volume: 0.8,
            keys,
            leader: " ".into(),
            leader_keys: {
                let mut m = HashMap::new();
                m.insert("fy".into(), "search_youtube".into());
                m.insert("fso".into(), "search_soundcloud".into());
                m.insert("ff".into(), "search_files".into());
                m
            },
            picker_keys,
        }
    }
}

pub fn load_config() -> Config {
    let mut cfg = Config::default();
    let Some(path) = config_path() else {
        return cfg;
    };
    if !path.exists() {
        return cfg;
    }

    if let Err(e) = apply_lua(&mut cfg, &path) {
        eprintln!("failed to load {}: {e}", path.display());
    }
    cfg
}

fn config_path() -> Option<PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("musicli");
    dir.push("init.lua");
    Some(dir)
}

pub fn apply_lua(cfg: &mut Config, path: &PathBuf) -> mlua::Result<()> {
    let lua = Lua::new();
    let theme_tbl = lua.create_table()?;
    let cfg_ptr = cfg as *mut Config;

    let set = lua.create_function(move |_, table: Table| {
        let cfg = unsafe { &mut *cfg_ptr };
        for pair in table.pairs::<String, Table>() {
            let (name, spec) = pair?;
            let mut hl = cfg.theme.get(&name);
            if let Ok(fg) = spec.get::<String>("fg") {
                hl.fg = parse_color(&fg);
            }
            if let Ok(bg) = spec.get::<String>("bg") {
                hl.bg = parse_color(&bg);
            }
            if spec.get::<bool>("bold").unwrap_or(false) {
                hl.modifier |= Modifier::BOLD;
            }
            if spec.get::<bool>("italic").unwrap_or(false) {
                hl.modifier |= Modifier::ITALIC;
            }
            if spec.get::<bool>("dim").unwrap_or(false) {
                hl.modifier |= Modifier::DIM;
            }
            cfg.theme.set(&name, hl);
        }
        Ok(())
    })?;
    theme_tbl.set("set", set)?;

    let keys_tbl = lua.create_table()?;
    let cfg_ptr2 = cfg as *mut Config;
    let map = lua.create_function(move |_, table: Table| {
        let cfg = unsafe { &mut *cfg_ptr2 };
        for pair in table.pairs::<String, String>() {
            let (key, action) = pair?;

            let key = normalize_key(&key);
            if action.starts_with("picker_") {
                cfg.picker_keys.insert(key, action);
            } else if let Some(rest) = key.strip_prefix("<leader>") {
                cfg.leader_keys.insert(rest.to_string(), action);
            } else {
                cfg.keys.insert(key, action);
            }
        }
        Ok(())
    })?;

    keys_tbl.set("map", map)?;

    let globals = lua.globals();
    globals.set("theme", theme_tbl)?;
    globals.set("keys", keys_tbl)?;
    globals.set("volume", cfg.volume)?;

    lua.load(path.as_path()).exec()?;

    if let Ok(v) = globals.get::<f32>("volume") {
        cfg.volume = v.clamp(0.0, 1.0);
    }
    if let Ok(v) = globals.get::<String>("leader") {
        if !v.is_empty() {
            cfg.leader = v;
        }
    }
    Ok(())
}

fn normalize_key(k: &str) -> String {
    let k = k.trim();
    if let Some(rest) = k.strip_prefix("<C-").and_then(|s| s.strip_suffix('>')) {
        return format!("c-{}", rest.to_ascii_lowercase());
    }
    k.to_string()
}
