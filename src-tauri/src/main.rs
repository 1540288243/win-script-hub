// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub category: String,
    pub description: String,
    pub auto_start: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub script_dir: String,
    pub scripts: Vec<ScriptInfo>,
    #[serde(default)]
    pub categories: Vec<String>,
    /// 关闭按钮行为: "to_tray" = 最小化到托盘, "quit" = 彻底关闭
    #[serde(default = "default_close_action")]
    pub close_action: String,
    /// 全局快捷键，格式如 "Ctrl+Shift+S"
    #[serde(default)]
    pub global_shortcut: String,
}

fn default_close_action() -> String {
    "to_tray".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\"))
            .join("MyScripts")
            .to_string_lossy()
            .to_string();

        AppConfig {
            script_dir: default_dir,
            scripts: vec![],
            categories: vec!["默认".to_string()],
            close_action: default_close_action(),
            global_shortcut: "Ctrl+Shift+S".to_string(),
        }
    }
}

fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("C:\\"))
        .join("win-script-hub");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
}

fn load_config() -> Result<AppConfig, String> {
    let config_path = get_config_path();
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppConfig::default())
    }
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("s_{}_{}", duration.as_millis(), rand_u32())
}

fn rand_u32() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    (hasher.finish() & 0xFFFFFFFF) as u32
}

// ============ 配置读写 ============
#[tauri::command]
fn load_config_cmd() -> Result<AppConfig, String> {
    load_config()
}

#[tauri::command]
fn save_config_cmd(config: AppConfig) -> Result<(), String> {
    save_config(&config)
}

// ============ 导入/导出 ============
#[tauri::command]
fn export_config(path: String) -> Result<(), String> {
    let config = load_config()?;
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_config(path: String) -> Result<AppConfig, String> {
    let content = fs::read_to_string(&path)
        .map_err(|e| e.to_string())?;
    let imported: AppConfig = serde_json::from_str(&content)
        .map_err(|e| format!("配置文件格式错误: {}", e))?;

    // 保存导入的配置
    save_config(&imported)?;

    // 创建分类文件夹
    for cat in &imported.categories {
        let cat_folder = PathBuf::from(&imported.script_dir).join(cat);
        fs::create_dir_all(&cat_folder).ok();
    }

    Ok(imported)
}

// ============ 扫描目录 ============
#[derive(Debug, Serialize)]
pub struct DiscoveredScript {
    pub name: String,
    pub path: String,
}

#[tauri::command]
fn scan_directory(dir_path: String) -> Result<Vec<DiscoveredScript>, String> {
    let mut scripts = Vec::new();
    let path = PathBuf::from(&dir_path);

    if !path.exists() {
        return Err("目录不存在".to_string());
    }

    fn scan_dir(dir: &PathBuf, scripts: &mut Vec<DiscoveredScript>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, scripts);
                } else if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "bat" || ext_str == "cmd" {
                        let name = path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "未知脚本".to_string());
                        scripts.push(DiscoveredScript {
                            name,
                            path: path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }

    scan_dir(&path, &mut scripts);
    Ok(scripts)
}

#[tauri::command]
fn import_script_from_path(source_path: String, category: String, config: AppConfig) -> Result<ScriptInfo, String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err("源文件不存在".to_string());
    }

    let file_name = source.file_name()
        .ok_or("无法获取文件名")?
        .to_string_lossy()
        .to_string();

    // 确保分类文件夹存在
    let cat_folder = PathBuf::from(&config.script_dir).join(&category);
    fs::create_dir_all(&cat_folder).map_err(|e| e.to_string())?;

    let dest_path = cat_folder.join(&file_name);

    // 复制文件
    fs::copy(&source, &dest_path).map_err(|e| e.to_string())?;

    let name = source.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "未知脚本".to_string());

    Ok(ScriptInfo {
        id: generate_id(),
        name,
        path: dest_path.to_string_lossy().to_string(),
        category,
        description: String::new(),
        auto_start: false,
    })
}

// ============ 脚本管理 ============
#[tauri::command]
fn add_script(script: ScriptInfo) -> Result<(), String> {
    let mut config = load_config()?;
    config.scripts.push(script);
    save_config(&config)
}

#[tauri::command]
fn update_script(script: ScriptInfo) -> Result<(), String> {
    let mut config = load_config()?;
    if let Some(pos) = config.scripts.iter().position(|s| s.id == script.id) {
        config.scripts[pos] = script;
        save_config(&config)
    } else {
        Err("Script not found".to_string())
    }
}

#[tauri::command]
fn delete_script(id: String, script_dir: String) -> Result<(), String> {
    let mut config = load_config()?;

    // 找到脚本并删除文件（如果文件在脚本目录内）
    if let Some(script) = config.scripts.iter().find(|s| s.id == id) {
        let path = PathBuf::from(&script.path);
        if path.exists() && path.to_string_lossy().starts_with(&script_dir) {
            fs::remove_file(&path).ok();
        }
    }

    config.scripts.retain(|s| s.id != id);
    save_config(&config)
}

// 只删除脚本文件（不删配置）
#[tauri::command]
fn delete_script_file(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.exists() {
        fs::remove_file(&p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// 移动脚本文件到新的分类文件夹
#[tauri::command]
fn move_script_file(old_path: String, new_category: String, script_dir: String) -> Result<String, String> {
    let source = PathBuf::from(&old_path);
    if !source.exists() {
        return Err("源文件不存在".to_string());
    }

    let file_name = source.file_name()
        .ok_or("无法获取文件名")?
        .to_string_lossy()
        .to_string();

    // 目标分类文件夹
    let cat_folder = PathBuf::from(&script_dir).join(&new_category);
    fs::create_dir_all(&cat_folder).map_err(|e| e.to_string())?;

    let dest_path = cat_folder.join(&file_name);

    // 如果目标文件已存在，先删除
    if dest_path.exists() {
        fs::remove_file(&dest_path).map_err(|e| e.to_string())?;
    }

    // 移动文件，跨磁盘则复制+删除
    match fs::rename(&source, &dest_path) {
        Ok(_) => {}
        Err(_) => {
            // 跨磁盘无法 rename，改用复制+删除
            fs::copy(&source, &dest_path).map_err(|e| e.to_string())?;
            fs::remove_file(&source).map_err(|e| e.to_string())?;
        }
    }

    Ok(dest_path.to_string_lossy().to_string())
}

// 打开分类文件夹
#[tauri::command]
fn open_category_folder(category: String, script_dir: String) -> Result<(), String> {
    let folder = PathBuf::from(&script_dir).join(&category);
    // 确保文件夹存在
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;

    // Windows 用 explorer 打开
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(folder.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn run_script(path: String) -> Result<String, String> {
    let path_obj = PathBuf::from(&path);
    if !path_obj.exists() {
        return Err("Script file not found".to_string());
    }

    let extension = path_obj.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "bat" | "cmd" => {
            // 使用 start 命令弹出新窗口运行，支持中文路径和带空格路径
            Command::new("cmd")
                .args(["/c", "start", "", &path])
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok("Script started".to_string())
        }
        _ => Err("Unsupported script type".to_string())
    }
}

// ============ 分类管理 ============
#[tauri::command]
fn add_category(name: String) -> Result<(), String> {
    let mut config = load_config()?;
    if config.categories.contains(&name) {
        return Err("分类已存在".to_string());
    }
    // 创建分类文件夹
    let cat_folder = PathBuf::from(&config.script_dir).join(&name);
    fs::create_dir_all(&cat_folder).ok();
    config.categories.push(name);
    save_config(&config)
}

#[tauri::command]
fn update_category(old_name: String, new_name: String) -> Result<(), String> {
    let mut config = load_config()?;
    if let Some(pos) = config.categories.iter().position(|c| *c == old_name) {
        // 重命名文件夹
        let old_folder = PathBuf::from(&config.script_dir).join(&old_name);
        let new_folder = PathBuf::from(&config.script_dir).join(&new_name);
        if old_folder.exists() {
            fs::rename(&old_folder, &new_folder).ok();
        }
        // 更新分类名
        config.categories[pos] = new_name.clone();
        // 同时更新所有脚本的路径和分类
        for script in &mut config.scripts {
            if script.category == old_name {
                script.category = new_name.clone();
                if script.path.starts_with(old_folder.to_string_lossy().as_ref()) {
                    let file_name = PathBuf::from(&script.path).file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    script.path = new_folder.join(&file_name).to_string_lossy().to_string();
                }
            }
        }
        save_config(&config)
    } else {
        Err("分类不存在".to_string())
    }
}

#[tauri::command]
fn delete_category(name: String) -> Result<(), String> {
    if name == "默认" {
        return Err("默认分类不能删除".to_string());
    }
    let mut config = load_config()?;
    let _cat_folder = PathBuf::from(&config.script_dir).join(&name);
    // 将该分类的脚本移到"默认"
    for script in &mut config.scripts {
        if script.category == name {
            script.category = "默认".to_string();
        }
    }
    config.categories.retain(|c| *c != name);
    save_config(&config)
}

// ============ 文件浏览器 ============
/// 将 GBK 编码的字节转为 UTF-8 字符串
fn gbk_to_string(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            result.push(b as char);
            i += 1;
        } else if i + 1 < bytes.len() {
            let hi = b as u16;
            let lo = bytes[i + 1] as u16;
            let gbk_code = (hi << 8) | lo;
            if let Some(c) = gbk_to_unicode(gbk_code) {
                result.push(c);
            }
            i += 2;
        } else {
            result.push(b as char);
            i += 1;
        }
    }
    result
}

fn gbk_to_unicode(gbk: u16) -> Option<char> {
    char::from_u32(gbk as u32)
}

#[tauri::command]
fn browse_folder() -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-Command",
            r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description = '选择脚本文件夹'; $f.ShowDialog() | Out-Null; $f.SelectedPath"#])
        .output()
        .map_err(|e| e.to_string())?;

    let path = match String::from_utf8(output.stdout.to_vec()) {
        Ok(s) => s.trim().to_string(),
        Err(_) => gbk_to_string(&output.stdout).trim().to_string(),
    };

    if path.is_empty() {
        Err("No folder selected".to_string())
    } else {
        Ok(path)
    }
}

#[tauri::command]
fn browse_file() -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-Command",
            r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = '批处理脚本 (*.bat;*.cmd)|*.bat;*.cmd|All Files (*.*)|*.*'; $f.ShowDialog() | Out-Null; $f.FileName"#])
        .output()
        .map_err(|e| e.to_string())?;

    let path = match String::from_utf8(output.stdout.to_vec()) {
        Ok(s) => s.trim().to_string(),
        Err(_) => gbk_to_string(&output.stdout).trim().to_string(),
    };

    if path.is_empty() {
        Err("No file selected".to_string())
    } else {
        Ok(path)
    }
}

#[tauri::command]
fn check_file_exists(path: String) -> bool {
    PathBuf::from(&path).exists()
}

// 文件保存对话框
#[tauri::command]
fn browse_save_file() -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-Command",
            r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.SaveFileDialog; $f.Filter = 'JSON Files (*.json)|*.json|All Files (*.*)|*.*'; $f.DefaultExt = 'json'; $f.ShowDialog() | Out-Null; $f.FileName"#])
        .output()
        .map_err(|e| e.to_string())?;

    let path = match String::from_utf8(output.stdout.to_vec()) {
        Ok(s) => s.trim().to_string(),
        Err(_) => gbk_to_string(&output.stdout).trim().to_string(),
    };

    if path.is_empty() {
        Err("No file selected".to_string())
    } else {
        Ok(path)
    }
}

// 文件打开对话框
#[tauri::command]
fn browse_open_json_file() -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-Command",
            r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = 'JSON Files (*.json)|*.json|All Files (*.*)|*.*'; $f.ShowDialog() | Out-Null; $f.FileName"#])
        .output()
        .map_err(|e| e.to_string())?;

    let path = match String::from_utf8(output.stdout.to_vec()) {
        Ok(s) => s.trim().to_string(),
        Err(_) => gbk_to_string(&output.stdout).trim().to_string(),
    };

    if path.is_empty() {
        Err("No file selected".to_string())
    } else {
        Ok(path)
    }
}

#[tauri::command]
fn create_category_folder(script_dir: String, category: String) -> Result<(), String> {
    let folder = PathBuf::from(&script_dir).join(&category);
    fs::create_dir_all(&folder).map_err(|e| e.to_string())
}

// ============ 全局快捷键注册 ============
#[tauri::command]
fn register_global_shortcut(shortcut: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    // 解析快捷键字符串，如 "Ctrl+Shift+S"
    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.is_empty() {
        return Err("快捷键格式错误".to_string());
    }

    // 构建快捷键
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let mut mods = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match part.trim().to_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "ALT" => mods |= Modifiers::ALT,
            "WIN" | "WINDOWS" => mods |= Modifiers::SUPER,
            // 功能键
            "F1" => key_code = Some(Code::F1),
            "F2" => key_code = Some(Code::F2),
            "F3" => key_code = Some(Code::F3),
            "F4" => key_code = Some(Code::F4),
            "F5" => key_code = Some(Code::F5),
            "F6" => key_code = Some(Code::F6),
            "F7" => key_code = Some(Code::F7),
            "F8" => key_code = Some(Code::F8),
            "F9" => key_code = Some(Code::F9),
            "F10" => key_code = Some(Code::F10),
            "F11" => key_code = Some(Code::F11),
            "F12" => key_code = Some(Code::F12),
            // 字母键
            "A" => key_code = Some(Code::KeyA),
            "B" => key_code = Some(Code::KeyB),
            "C" => key_code = Some(Code::KeyC),
            "D" => key_code = Some(Code::KeyD),
            "E" => key_code = Some(Code::KeyE),
            "F" => key_code = Some(Code::KeyF),
            "G" => key_code = Some(Code::KeyG),
            "H" => key_code = Some(Code::KeyH),
            "I" => key_code = Some(Code::KeyI),
            "J" => key_code = Some(Code::KeyJ),
            "K" => key_code = Some(Code::KeyK),
            "L" => key_code = Some(Code::KeyL),
            "M" => key_code = Some(Code::KeyM),
            "N" => key_code = Some(Code::KeyN),
            "O" => key_code = Some(Code::KeyO),
            "P" => key_code = Some(Code::KeyP),
            "Q" => key_code = Some(Code::KeyQ),
            "R" => key_code = Some(Code::KeyR),
            "S" => key_code = Some(Code::KeyS),
            "T" => key_code = Some(Code::KeyT),
            "U" => key_code = Some(Code::KeyU),
            "V" => key_code = Some(Code::KeyV),
            "W" => key_code = Some(Code::KeyW),
            "X" => key_code = Some(Code::KeyX),
            "Y" => key_code = Some(Code::KeyY),
            "Z" => key_code = Some(Code::KeyZ),
            // 数字键
            "0" => key_code = Some(Code::Digit0),
            "1" => key_code = Some(Code::Digit1),
            "2" => key_code = Some(Code::Digit2),
            "3" => key_code = Some(Code::Digit3),
            "4" => key_code = Some(Code::Digit4),
            "5" => key_code = Some(Code::Digit5),
            "6" => key_code = Some(Code::Digit6),
            "7" => key_code = Some(Code::Digit7),
            "8" => key_code = Some(Code::Digit8),
            "9" => key_code = Some(Code::Digit9),
            // 其他常用键
            "SPACE" => key_code = Some(Code::Space),
            "ENTER" | "RETURN" => key_code = Some(Code::Enter),
            "TAB" => key_code = Some(Code::Tab),
            "ESCAPE" | "ESC" => key_code = Some(Code::Escape),
            _ => {}
        }
    }

    let Some(key) = key_code else {
        return Err("无效的快捷键".to_string());
    };

    let shortcut_obj = Shortcut::new(Some(mods), key);

    // 取消之前可能已注册的相同快捷键
    let _ = app_handle.global_shortcut().unregister(shortcut_obj.clone());

    // 注册新快捷键
    let handle_clone = app_handle.clone();
    app_handle.global_shortcut().on_shortcut(shortcut_obj, move |_app, _shortcut, _event| {
        // 快捷键被触发，显示主窗口
        if let Some(window) = handle_clone.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.unminimize();
        }
    }).map_err(|e| e.to_string())?;

    Ok(())
}

// ============ 显示/隐藏窗口命令 ============
#[tauri::command]
fn hide_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn show_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.unminimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            // 配置
            load_config_cmd,
            save_config_cmd,
            export_config,
            import_config,
            // 目录扫描
            scan_directory,
            import_script_from_path,
            create_category_folder,
            // 脚本管理
            add_script,
            update_script,
            delete_script,
            delete_script_file,
            move_script_file,
            run_script,
            // 文件浏览器
            browse_folder,
            browse_file,
            check_file_exists,
            browse_save_file,
            browse_open_json_file,
            // 分类管理
            add_category,
            update_category,
            delete_category,
            open_category_folder,
            // 窗口控制
            hide_window,
            show_window,
            register_global_shortcut,
        ])
        .setup(|app| {
            // 启动时初始化分类文件夹
            if let Ok(config) = load_config() {
                for cat in &config.categories {
                    let folder = PathBuf::from(&config.script_dir).join(cat);
                    fs::create_dir_all(&folder).ok();
                }

                // 注册全局快捷键
                if !config.global_shortcut.is_empty() {
                    let handle = app.handle().clone();
                    let shortcut = config.global_shortcut.clone();
                    std::thread::spawn(move || {
                        // 延迟注册，等应用完全启动
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if let Err(e) = register_global_shortcut_internal(&handle, &shortcut) {
                            eprintln!("注册快捷键失败: {}", e);
                        }
                    });
                }
            }

            // 创建系统托盘
            let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "彻底退出", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Win脚本中心")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.unminimize();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 双击托盘图标显示窗口
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // 拦截窗口关闭事件
            if let WindowEvent::CloseRequested { api, .. } = event {
                let config = load_config().unwrap_or_default();

                if config.close_action == "to_tray" {
                    // 最小化到托盘
                    api.prevent_close();
                    let _ = window.hide();
                }
                // 如果是 quit，直接关闭
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 内部快捷键注册函数
fn register_global_shortcut_internal(app_handle: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.is_empty() {
        return Err("快捷键格式错误".to_string());
    }

    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let mut mods = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match part.trim().to_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "ALT" => mods |= Modifiers::ALT,
            "WIN" | "WINDOWS" => mods |= Modifiers::SUPER,
            "F1" => key_code = Some(Code::F1),
            "F2" => key_code = Some(Code::F2),
            "F3" => key_code = Some(Code::F3),
            "F4" => key_code = Some(Code::F4),
            "F5" => key_code = Some(Code::F5),
            "F6" => key_code = Some(Code::F6),
            "F7" => key_code = Some(Code::F7),
            "F8" => key_code = Some(Code::F8),
            "F9" => key_code = Some(Code::F9),
            "F10" => key_code = Some(Code::F10),
            "F11" => key_code = Some(Code::F11),
            "F12" => key_code = Some(Code::F12),
            "A" => key_code = Some(Code::KeyA),
            "B" => key_code = Some(Code::KeyB),
            "C" => key_code = Some(Code::KeyC),
            "D" => key_code = Some(Code::KeyD),
            "E" => key_code = Some(Code::KeyE),
            "F" => key_code = Some(Code::KeyF),
            "G" => key_code = Some(Code::KeyG),
            "H" => key_code = Some(Code::KeyH),
            "I" => key_code = Some(Code::KeyI),
            "J" => key_code = Some(Code::KeyJ),
            "K" => key_code = Some(Code::KeyK),
            "L" => key_code = Some(Code::KeyL),
            "M" => key_code = Some(Code::KeyM),
            "N" => key_code = Some(Code::KeyN),
            "O" => key_code = Some(Code::KeyO),
            "P" => key_code = Some(Code::KeyP),
            "Q" => key_code = Some(Code::KeyQ),
            "R" => key_code = Some(Code::KeyR),
            "S" => key_code = Some(Code::KeyS),
            "T" => key_code = Some(Code::KeyT),
            "U" => key_code = Some(Code::KeyU),
            "V" => key_code = Some(Code::KeyV),
            "W" => key_code = Some(Code::KeyW),
            "X" => key_code = Some(Code::KeyX),
            "Y" => key_code = Some(Code::KeyY),
            "Z" => key_code = Some(Code::KeyZ),
            "0" => key_code = Some(Code::Digit0),
            "1" => key_code = Some(Code::Digit1),
            "2" => key_code = Some(Code::Digit2),
            "3" => key_code = Some(Code::Digit3),
            "4" => key_code = Some(Code::Digit4),
            "5" => key_code = Some(Code::Digit5),
            "6" => key_code = Some(Code::Digit6),
            "7" => key_code = Some(Code::Digit7),
            "8" => key_code = Some(Code::Digit8),
            "9" => key_code = Some(Code::Digit9),
            "SPACE" => key_code = Some(Code::Space),
            "ENTER" | "RETURN" => key_code = Some(Code::Enter),
            "TAB" => key_code = Some(Code::Tab),
            "ESCAPE" | "ESC" => key_code = Some(Code::Escape),
            _ => {}
        }
    }

    let Some(key) = key_code else {
        return Err("无效的快捷键".to_string());
    };

    let shortcut_obj = Shortcut::new(Some(mods), key);
    let handle_clone = app_handle.clone();

    app_handle.global_shortcut().on_shortcut(shortcut_obj, move |_app, _shortcut, _event| {
        if let Some(window) = handle_clone.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.unminimize();
        }
    }).map_err(|e| e.to_string())?;

    Ok(())
}
