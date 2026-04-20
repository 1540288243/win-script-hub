// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub category: String,
    pub description: String,
    pub auto_start: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub script_dir: String,
    pub scripts: Vec<ScriptInfo>,
    #[serde(default)]
    pub categories: Vec<String>,
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
            // start 语法: start "窗口标题" "脚本路径"
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
    match gbk {
        _ if gbk >= 0xB0A1 && gbk <= 0xF7FE => {
            char::from_u32(gbk as u32)
        }
        _ => char::from_u32(gbk as u32),
    }
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
        ])
        .setup(|_app| {
            // 启动时初始化分类文件夹
            if let Ok(config) = load_config() {
                for cat in &config.categories {
                    let folder = PathBuf::from(&config.script_dir).join(cat);
                    fs::create_dir_all(&folder).ok();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
