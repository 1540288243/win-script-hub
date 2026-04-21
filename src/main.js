// 全局状态
let config = null;
let currentCategory = 'all';

// ============ 初始化 ============
document.addEventListener('DOMContentLoaded', async () => {
    showLoading(true);
    try {
        config = await invoke('load_config_cmd');
        // 确保默认分类存在
        if (!config.categories || config.categories.length === 0) {
            config.categories = ['默认'];
        }
        // 确保 close_action 字段存在（兼容旧配置）
        if (!config.close_action) {
            config.close_action = 'to_tray';
        }
        // 更新脚本目录显示
        const dirEl = document.getElementById('scriptDir');
        dirEl.textContent = config.script_dir || '未设置';
        dirEl.title = config.script_dir || '';
        // 初始化脚本目录
        if (config.script_dir) {
            for (const cat of config.categories) {
                await invoke('create_category_folder', { scriptDir: config.script_dir, category: cat });
            }
        }
        renderCategories();
        renderScripts();
    } catch (err) {
        showToast('加载配置失败: ' + err, 'error');
    } finally {
        showLoading(false);
    }
    bindEvents();
});

// ============ 事件绑定 ============
function bindEvents() {
    // 目录浏览
    document.getElementById('btnBrowseDir').addEventListener('click', browseDirectory);

    // 添加脚本
    document.getElementById('btnAddScript').addEventListener('click', () => openScriptModal());

    // 添加分类
    document.getElementById('btnAddCategory').addEventListener('click', () => openCategoryModal());

    // 扫描目录
    document.getElementById('btnScanDir').addEventListener('click', scanDirectory);

    // 导入配置
    document.getElementById('btnImport').addEventListener('click', importConfig);

    // 导出配置
    document.getElementById('btnExport').addEventListener('click', exportConfig);

    // 设置
    document.getElementById('btnSettings').addEventListener('click', openSettings);
    document.getElementById('settingsForm').addEventListener('submit', handleSettingsSubmit);
    document.getElementById('btnCancelSettings').addEventListener('click', closeSettingsModal);

    // 脚本表单
    document.getElementById('scriptForm').addEventListener('submit', handleScriptSubmit);

    // 分类表单
    document.getElementById('categoryForm').addEventListener('submit', handleCategorySubmit);

    // 浏览文件
    document.getElementById('btnBrowseFile').addEventListener('click', browseFile);

    // 取消按钮
    document.getElementById('btnCancel').addEventListener('click', closeScriptModal);
    document.getElementById('btnCancelCategory').addEventListener('click', closeCategoryModal);

    // 点击弹窗背景关闭
    document.querySelectorAll('.modal').forEach(modal => {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                modal.classList.remove('visible');
            }
        });
    });

    // 分类右键菜单
    document.getElementById('categoryList').addEventListener('contextmenu', (e) => {
        const li = e.target.closest('li[data-category]');
        if (li && li.dataset.category !== 'all') {
            e.preventDefault();
            showCategoryContextMenu(li.dataset.category, e.clientX, e.clientY);
        }
    });

    // 点击其他地方关闭右键菜单
    document.addEventListener('click', hideContextMenu);
}

// ============ 分类右键菜单 ============
let currentContextCategory = null;

function showCategoryContextMenu(category, x, y) {
    hideContextMenu();
    currentContextCategory = category;

    const menu = document.createElement('div');
    menu.className = 'context-menu';
    menu.id = 'categoryContextMenu';
    menu.innerHTML = `
        <div class="context-menu-item" onclick="openCategoryFolder('${escapeHtml(category)}')">
            📂 打开文件夹
        </div>
    `;

    menu.style.left = x + 'px';
    menu.style.top = y + 'px';
    document.body.appendChild(menu);

    // 确保菜单在可视区域内
    const rect = menu.getBoundingClientRect();
    if (rect.right > window.innerWidth) {
        menu.style.left = (x - rect.width) + 'px';
    }
    if (rect.bottom > window.innerHeight) {
        menu.style.top = (y - rect.height) + 'px';
    }
}

function hideContextMenu() {
    const menu = document.getElementById('categoryContextMenu');
    if (menu) {
        menu.remove();
    }
    currentContextCategory = null;
}

async function openCategoryFolder(category) {
    hideContextMenu();
    if (!config.script_dir) {
        showToast('请先设置脚本目录', 'error');
        return;
    }
    try {
        await invoke('open_category_folder', {
            category: category,
            scriptDir: config.script_dir
        });
    } catch (err) {
        showToast('打开文件夹失败: ' + err, 'error');
    }
}

// ============ Tauri 调用封装 ============
async function invoke(cmd, args = {}) {
    return await window.__TAURI__.core.invoke(cmd, args);
}

// ============ 目录浏览 ============
async function browseDirectory() {
    try {
        const path = await invoke('browse_folder');
        if (path && path !== 'No folder selected') {
            const dirEl = document.getElementById('scriptDir');
            dirEl.textContent = path;
            dirEl.title = path;
            config.script_dir = path;
            // 确保所有分类文件夹存在
            for (const cat of config.categories) {
                await invoke('create_category_folder', { scriptDir: path, category: cat });
            }
            await saveConfig();
            showToast('脚本目录已设置', 'success');
        }
    } catch (err) {
        showToast('选择目录失败: ' + err, 'error');
    }
}

// ============ 文件浏览 ============
async function browseFile() {
    try {
        const path = await invoke('browse_file');
        if (path && path !== 'No file selected') {
            document.getElementById('scriptPath').value = path;
        }
    } catch (err) {
        showToast('选择文件失败: ' + err, 'error');
    }
}

// ============ 分类管理 ============
function renderCategories() {
    const list = document.getElementById('categoryList');
    let html = `<li class="${currentCategory === 'all' ? 'active' : ''}" data-category="all">全部脚本</li>`;

    config.categories.forEach(cat => {
        const count = config.scripts.filter(s => s.category === cat).length;
        html += `
            <li class="category-item ${currentCategory === cat ? 'active' : ''}" data-category="${cat}">
                <span class="cat-name">${cat}<span class="cat-count"> (${count})</span></span>
                <div class="cat-actions">
                    <button class="cat-btn" onclick="editCategory('${cat}')" title="编辑">✏️</button>
                    <button class="cat-btn" onclick="deleteCategory('${cat}')" title="删除">🗑️</button>
                </div>
            </li>
        `;
    });

    list.innerHTML = html;

    // 绑定分类点击事件
    list.querySelectorAll('li[data-category]').forEach(li => {
        li.addEventListener('click', (e) => {
            if (e.target.closest('.cat-actions')) return;
            switchCategory(li.dataset.category);
        });
    });

    // 更新分类选择器
    const categorySelect = document.getElementById('scriptCategory');
    categorySelect.innerHTML = config.categories.map(cat =>
        `<option value="${cat}">${cat}</option>`
    ).join('');
}

async function switchCategory(category) {
    currentCategory = category;
    document.getElementById('currentView').textContent =
        category === 'all' ? '全部脚本' : category;
    renderCategories();
    renderScripts();
}

function openCategoryModal(editName = null) {
    const modal = document.getElementById('categoryModal');
    const form = document.getElementById('categoryForm');
    const input = document.getElementById('categoryName');

    modal.querySelector('h3').textContent = editName ? '编辑分类' : '添加分类';
    form.onsubmit = (e) => handleCategorySubmit(e, editName);
    input.value = editName || '';
    input.dataset.oldName = editName || '';

    modal.classList.add('visible');
    input.focus();
}

async function handleCategorySubmit(e, editName = null) {
    e.preventDefault();
    const name = document.getElementById('categoryName').value.trim();
    if (!name) return;

    try {
        if (editName) {
            await invoke('update_category', { oldName: editName, newName: name });
            showToast('分类已重命名', 'success');
        } else {
            await invoke('add_category', { name });
            showToast('分类已添加', 'success');
        }
        config = await invoke('load_config_cmd');
        renderCategories();
        closeCategoryModal();
    } catch (err) {
        showToast(err, 'error');
    }
}

async function editCategory(name) {
    openCategoryModal(name);
}

async function deleteCategory(name) {
    if (!confirm(`确定要删除分类「${name}」吗？该分类下的脚本将移至「默认」。`)) return;

    try {
        await invoke('delete_category', { name });
        if (currentCategory === name) {
            currentCategory = 'all';
            document.getElementById('currentView').textContent = '全部脚本';
        }
        config = await invoke('load_config_cmd');
        renderCategories();
        renderScripts();
        showToast('分类已删除', 'success');
    } catch (err) {
        showToast(err, 'error');
    }
}

// ============ 脚本管理 ============
function renderScripts() {
    const list = document.getElementById('scriptList');
    const empty = document.getElementById('emptyState');

    let scripts = config.scripts;
    if (currentCategory !== 'all') {
        scripts = scripts.filter(s => s.category === currentCategory);
    }

    if (scripts.length === 0) {
        list.innerHTML = '';
        empty.classList.add('visible');
        return;
    }

    empty.classList.remove('visible');

    list.innerHTML = scripts.map(script => `
        <div class="script-card" data-id="${script.id}">
            <div class="script-card-header">
                <div class="script-icon">BAT</div>
                <div class="script-info">
                    <h4>${escapeHtml(script.name)}</h4>
                    <span class="category">${escapeHtml(script.category)}</span>
                    ${script.auto_start ? '<span class="auto-start-badge">自启</span>' : ''}
                </div>
            </div>
            ${script.description ? `<p class="description">${escapeHtml(script.description)}</p>` : ''}
            <div class="actions">
                <button class="btn-run" onclick="runScript('${script.id}')">▶️ 运行</button>
                <button class="btn-edit" onclick="editScript('${script.id}')">✏️ 编辑</button>
                <button class="btn-delete" onclick="deleteScript('${script.id}')">🗑️ 删除</button>
            </div>
        </div>
    `).join('');
}

function openScriptModal(script = null) {
    const modal = document.getElementById('scriptModal');
    const form = document.getElementById('scriptForm');

    document.getElementById('modalTitle').textContent = script ? '编辑脚本' : '添加脚本';
    document.getElementById('scriptId').value = script ? script.id : '';
    document.getElementById('scriptName').value = script ? script.name : '';
    document.getElementById('scriptPath').value = script ? script.path : '';
    document.getElementById('scriptCategory').value = script ? script.category : (config.categories[0] || '默认');
    document.getElementById('scriptDesc').value = script ? script.description : '';
    document.getElementById('scriptAutoStart').checked = script ? script.auto_start : false;

    modal.classList.add('visible');
    document.getElementById('scriptName').focus();
}

function closeScriptModal() {
    document.getElementById('scriptModal').classList.remove('visible');
}

function closeCategoryModal() {
    document.getElementById('categoryModal').classList.remove('visible');
}

async function handleScriptSubmit(e) {
    e.preventDefault();

    const id = document.getElementById('scriptId').value;
    const name = document.getElementById('scriptName').value.trim();
    const path = document.getElementById('scriptPath').value.trim();
    const category = document.getElementById('scriptCategory').value;
    const description = document.getElementById('scriptDesc').value.trim();
    const auto_start = document.getElementById('scriptAutoStart').checked;

    if (!name || !path) {
        showToast('请填写必填项', 'error');
        return;
    }

    // 检查文件是否存在
    const exists = await invoke('check_file_exists', { path });
    if (!exists) {
        showToast('脚本文件不存在', 'error');
        return;
    }

    try {
        let finalPath = path;

        // 新增脚本
        if (!id) {
            // 外部文件：复制到分类文件夹
            if (config.script_dir && !path.startsWith(config.script_dir)) {
                const scriptInfo = await invoke('import_script_from_path', {
                    sourcePath: path,
                    category: category,
                    config: config
                });
                config.scripts.push(scriptInfo);
                await saveConfig();
                showToast('脚本已添加', 'success');
            } else {
                // 已在目录内，直接添加
                const newScript = {
                    id: 's_' + Date.now(),
                    name,
                    path: finalPath,
                    category,
                    description,
                    auto_start
                };
                await invoke('add_script', { script: newScript });
                config.scripts.push(newScript);
                await saveConfig();
                showToast('脚本已添加', 'success');
            }
        } else {
            // 编辑现有脚本
            const existingScript = config.scripts.find(s => s.id === id);
            if (!existingScript) {
                showToast('脚本不存在', 'error');
                return;
            }

            const oldCategory = existingScript.category;
            const oldPath = existingScript.path;

            // 如果选择了外部文件（不在脚本目录内），复制到分类文件夹，删旧文件
            if (config.script_dir && !path.startsWith(config.script_dir)) {
                const scriptInfo = await invoke('import_script_from_path', {
                    sourcePath: path,
                    category: category,
                    config: config
                });
                // 删除旧文件（如果在脚本目录内）
                if (oldPath.startsWith(config.script_dir)) {
                    await invoke('delete_script_file', { path: oldPath });
                }
                // 从配置中移除旧脚本，添加新脚本
                config.scripts = config.scripts.filter(s => s.id !== id);
                config.scripts.push(scriptInfo);
                await saveConfig();
                renderScripts();
                renderCategories();
                closeScriptModal();
                showToast('脚本已更新', 'success');
                return;
            }

            // 在脚本目录内：处理分类变更（移动文件）
            if (oldCategory !== category && config.script_dir && oldPath.startsWith(config.script_dir)) {
                finalPath = await invoke('move_script_file', {
                    oldPath: oldPath,
                    newCategory: category,
                    scriptDir: config.script_dir
                });
            }

            const updatedScript = {
                id,
                name,
                path: finalPath,
                category,
                description,
                auto_start
            };
            await invoke('update_script', { script: updatedScript });

            const idx = config.scripts.findIndex(s => s.id === id);
            if (idx >= 0) config.scripts[idx] = updatedScript;
            await saveConfig();
            showToast('脚本已更新', 'success');
        }

        renderScripts();
        renderCategories();
        closeScriptModal();
    } catch (err) {
        showToast('保存失败: ' + err, 'error');
    }
}

async function editScript(id) {
    const script = config.scripts.find(s => s.id === id);
    if (script) {
        openScriptModal(script);
    }
}

async function deleteScript(id) {
    const script = config.scripts.find(s => s.id === id);
    if (!confirm(`确定要删除脚本「${script?.name}」吗？`)) return;

    try {
        await invoke('delete_script', { id, scriptDir: config.script_dir });
        config.scripts = config.scripts.filter(s => s.id !== id);
        renderScripts();
        renderCategories();
        showToast('脚本已删除', 'success');
    } catch (err) {
        showToast('删除失败: ' + err, 'error');
    }
}

async function runScript(id) {
    const script = config.scripts.find(s => s.id === id);
    if (!script) return;

    try {
        await invoke('run_script', { path: script.path });
        showToast('脚本已启动', 'success');
    } catch (err) {
        showToast('运行失败: ' + err, 'error');
    }
}

// ============ 扫描目录 ============
async function scanDirectory() {
    if (!config.script_dir) {
        showToast('请先设置脚本目录', 'error');
        return;
    }

    showLoading(true);
    try {
        const discovered = await invoke('scan_directory', { dirPath: config.script_dir });

        if (discovered.length === 0) {
            showToast('目录中没有发现脚本', 'info');
            return;
        }

        // 检查哪些脚本已经在列表中
        const existingPaths = new Set(config.scripts.map(s => s.path));
        const newScripts = discovered.filter(d => !existingPaths.has(d.path));

        if (newScripts.length === 0) {
            showToast('所有脚本已导入，无需重复导入', 'info');
            return;
        }

        // 显示扫描结果弹窗
        showScanModal(newScripts);
    } catch (err) {
        showToast('扫描失败: ' + err, 'error');
    } finally {
        showLoading(false);
    }
}

function showScanModal(discovered) {
    // 创建扫描结果弹窗
    let modal = document.getElementById('scanModal');
    if (!modal) {
        modal = document.createElement('div');
        modal.id = 'scanModal';
        modal.className = 'modal scan-modal';
        document.body.appendChild(modal);
    }

    modal.innerHTML = `
        <div class="modal-content">
            <h3>发现 ${discovered.length} 个新脚本</h3>
            <div class="select-all-wrapper">
                <input type="checkbox" id="selectAllScan" checked>
                <label for="selectAllScan">全选</label>
            </div>
            <div class="scan-list" id="scanList">
                ${discovered.map((d, i) => `
                    <div class="scan-item">
                        <input type="checkbox" data-index="${i}" checked>
                        <div class="scan-item-info">
                            <div class="scan-item-name">${escapeHtml(d.name)}</div>
                            <div class="scan-item-path">${escapeHtml(d.path)}</div>
                        </div>
                    </div>
                `).join('')}
            </div>
            <div class="scan-actions">
                <select id="scanCategory" style="padding: 8px; border-radius: 6px; border: 1px solid var(--border);">
                    ${config.categories.map(c => `<option value="${c}">${c}</option>`).join('')}
                </select>
                <button class="btn-secondary" onclick="closeScanModal()">取消</button>
                <button class="btn-primary" onclick="importScanned()">导入选中</button>
            </div>
        </div>
    `;

    // 全选功能
    modal.querySelector('#selectAllScan').addEventListener('change', (e) => {
        modal.querySelectorAll('.scan-item input[type="checkbox"]').forEach(cb => {
            cb.checked = e.target.checked;
        });
    });

    modal.classList.add('visible');
}

async function importScanned() {
    const modal = document.getElementById('scanModal');
    const checkboxes = modal.querySelectorAll('.scan-item input[type="checkbox"]:checked');
    const category = document.getElementById('scanCategory').value;

    if (checkboxes.length === 0) {
        showToast('请选择要导入的脚本', 'error');
        return;
    }

    showLoading(true);
    let imported = 0;

    try {
        for (const cb of checkboxes) {
            const index = parseInt(cb.dataset.index);
            // 这里需要获取原始数据，实际项目中应该存储
            const scanList = modal.querySelectorAll('.scan-item');
            const name = scanList[index].querySelector('.scan-item-name').textContent;
            const path = scanList[index].querySelector('.scan-item-path').textContent;

            const newScript = {
                id: 's_' + Date.now() + '_' + Math.random().toString(36).substr(2, 5),
                name: name,
                path: path,
                category: category,
                description: '',
                auto_start: false
            };

            await invoke('add_script', { script: newScript });
            config.scripts.push(newScript);
            imported++;
        }

        await saveConfig();
        closeScanModal();
        renderScripts();
        renderCategories();
        showToast(`已导入 ${imported} 个脚本`, 'success');
    } catch (err) {
        showToast('导入失败: ' + err, 'error');
    } finally {
        showLoading(false);
    }
}

function closeScanModal() {
    const modal = document.getElementById('scanModal');
    if (modal) {
        modal.classList.remove('visible');
    }
}

// ============ 导入/导出 ============
async function exportConfig() {
    try {
        const filePath = await invoke('browse_save_file');
        if (!filePath || filePath === 'No file selected') return;

        await invoke('export_config', { path: filePath });
        showToast('配置已导出', 'success');
    } catch (err) {
        showToast('导出失败: ' + err, 'error');
    }
}

async function importConfig() {
    try {
        const filePath = await invoke('browse_open_json_file');
        if (!filePath || filePath === 'No file selected') return;

        if (!confirm('导入将覆盖当前所有配置，是否继续？')) return;

        const imported = await invoke('import_config', { path: filePath });
        config = imported;
        const dirEl = document.getElementById('scriptDir');
        dirEl.textContent = config.script_dir;
        dirEl.title = config.script_dir;
        renderCategories();
        renderScripts();
        showToast('配置已导入', 'success');
    } catch (err) {
        showToast('导入失败: ' + err, 'error');
    }
}

// ============ 工具函数 ============
async function saveConfig() {
    try {
        await invoke('save_config_cmd', { config: config });
    } catch (err) {
        console.error('保存配置失败:', err);
    }
}

function showLoading(show) {
    const indicator = document.getElementById('loadingIndicator');
    if (indicator) {
        indicator.style.display = show ? 'block' : 'none';
    }
}

function showToast(message, type = 'info') {
    const toast = document.getElementById('toast') || createToast();
    toast.textContent = message;
    toast.className = 'toast visible ' + type;

    setTimeout(() => {
        toast.classList.remove('visible');
    }, 3000);
}

function createToast() {
    const toast = document.createElement('div');
    toast.id = 'toast';
    toast.className = 'toast';
    document.body.appendChild(toast);
    return toast;
}

function escapeHtml(str) {
    if (!str) return '';
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

// ============ 设置面板 ============
function openSettings() {
    const modal = document.getElementById('settingsModal');
    document.getElementById('closeAction').value = config.close_action || 'to_tray';
    document.getElementById('globalShortcut').value = config.global_shortcut || 'Ctrl+Shift+S';
    modal.classList.add('visible');
}

function closeSettingsModal() {
    document.getElementById('settingsModal').classList.remove('visible');
}

async function handleSettingsSubmit(e) {
    e.preventDefault();

    const closeAction = document.getElementById('closeAction').value;
    const globalShortcut = document.getElementById('globalShortcut').value.trim();

    // 验证快捷键格式
    if (globalShortcut && !/^[A-Z0-9]+(\+[A-Z0-9]+)*$/i.test(globalShortcut)) {
        showToast('快捷键格式错误，请使用如 Ctrl+Shift+S 的格式', 'error');
        return;
    }

    try {
        // 更新配置
        config.close_action = closeAction;
        config.global_shortcut = globalShortcut;

        // 保存配置
        await saveConfig();

        // 注册全局快捷键
        if (globalShortcut) {
            try {
                await invoke('register_global_shortcut', { shortcut: globalShortcut });
                showToast('设置已保存，快捷键已注册', 'success');
            } catch (shortcutErr) {
                console.error('注册快捷键失败:', shortcutErr);
                showToast('设置已保存，但快捷键注册失败: ' + shortcutErr, 'error');
            }
        }

        closeSettingsModal();
    } catch (err) {
        showToast('保存设置失败: ' + err, 'error');
    }
}
