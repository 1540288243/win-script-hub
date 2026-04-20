# Win脚本中心

一个简洁的 Windows 批处理脚本管理工具，基于 Tauri 2 构建。

## 功能特性

- ⚡ **脚本管理** - 添加、编辑、删除 .bat/.cmd 脚本
- 📂 **分类管理** - 按项目/用途对脚本分组，支持右键打开文件夹
- ▶️ **快捷执行** - 一键运行脚本，弹出独立窗口
- ⚙️ **自定义目录** - 指定脚本存放位置，脚本自动复制到分类目录
- 🚀 **开机自启** - 可标记脚本为开机自启
- 📥 **配置导入/导出** - 重装系统前导出配置，防止数据丢失
- 🔍 **目录扫描** - 自动发现指定目录下的脚本

## 界面预览

界面采用深色侧边栏 + 卡片式布局，支持分类筛选、右键菜单（打开文件夹）、脚本运行等功能。

## 环境要求

- Windows 10/11
- WebView2 运行时（Windows 11 已内置，Windows 10 可[下载安装](https://developer.microsoft.com/microsoft-edge/webview2/)）

## 安装使用

### 方式一：直接运行（推荐）

下载 Release 中的 `Win脚本中心_x64-setup.exe`，运行安装即可。

### 方式二：自行编译

```powershell
# 1. 安装 Rust（Windows）
# 访问 https://rustup.rs 下载安装

# 2. 克隆项目
git clone https://github.com/1540288243/win-script-hub.git
cd win-script-hub

# 3. 开发模式运行
cd src-tauri
cargo tauri dev

# 4. 构建发布包
cargo tauri build
```

## 使用说明

1. **设置脚本目录** - 点击侧边栏「浏览」选择你存放脚本的文件夹
2. **添加脚本** - 点击右上角「添加脚本」，选择 .bat 或 .cmd 文件，脚本会自动复制到分类目录
3. **分类管理** - 在添加/编辑脚本时选择或创建分类，右键分类可打开对应文件夹
4. **运行脚本** - 点击卡片上的「运行」按钮
5. **开机自启** - 勾选「开机自启」，配合 Windows 任务计划程序实现
6. **配置备份** - 点击「导出配置」保存到文件，重装系统后可「导入配置」恢复

## 数据存储

- 配置文件：`%APPDATA%\win-script-hub\config.json`
- 脚本目录：用户自定义，默认为 `脚本目录/`

## 技术栈

- **后端**：Rust + Tauri 2
- **前端**：原生 HTML/CSS/JavaScript
- **打包**：NSIS 安装程序

## License

MIT
