# GameFloat（游窗速播）

游戏攻略悬浮窗。在游戏上方浮着若干**无边框、置顶**的小窗，用来显示攻略网页；顶部一个小胶囊按钮即可拖动窗口、切换网址、翻页、刷新或打开设置。

基于 **Tauri 2 + React 19 + TypeScript + Vite** 构建，托盘常驻，支持全局快捷键和自定义 JS 动作。

> **AI 制作**：本项目（代码、界面与文档）由 AI 生成并持续迭代。

---

## 功能

- **悬浮小窗**：无边框、始终置顶、不占任务栏；内容页铺满整个小窗。
- **胶囊按钮**：顶部居中，圆角半透明；按住可拖动窗口，点击展开下拉栏。
  - 下拉栏：网址输入（回车跳转）、上一页、下一页、刷新、打开设置、隐藏小窗。
  - 展开带「胶囊放大 + 内容淡入同时进行」的过渡动画。
- **全局快捷键**：每个小窗可设置「显示 / 隐藏」快捷键，以及若干「动作」快捷键；按下动作快捷键会在该小窗的网页里执行自定义 JavaScript。
- **设置窗口**：小窗列表、网址、快捷键录制、动作代码编辑（CodeMirror，按需加载）、显隐切换。
- **配置导入 / 导出**：以 JSON 通过剪贴板导入导出小窗配置。
- **托盘菜单**：每个小窗的显示 / 隐藏、动作、打开设置、退出。
- **单实例**：重复启动会唤起已运行实例的设置窗口，不会开出第二个进程。

## 技术栈

| 层 | 选型 |
| --- | --- |
| 桌面外壳 | Tauri 2（`unstable`、`tray-icon`、`image-png`） |
| 前端 | React 19、TypeScript、Vite 7 |
| 代码编辑器 | CodeMirror（`@uiw/react-codemirror`） |
| 后端 | Rust（serde） |
| 插件 | `global-shortcut`、`clipboard-manager`、`single-instance` |

## 环境要求

- **Windows 10 / 11**（当前仅打包 NSIS 安装包）
- **Node.js** 20+ 与 npm
- **Rust**（stable 工具链）
- **WebView2 Runtime**（Windows 11 自带；Windows 10 通常由 Edge 提供）

## 快速开始

```bash
# 安装前端依赖
npm install

# 开发模式（启动 Vite 开发服务器 + Tauri 应用，热更新）
npm run tauri:dev

# 构建前端产物到 dist/
npm run web:build

# 类型检查
npm run typecheck
```

## 打包

```bash
# 标准 Tauri 打包（生成 NSIS 安装包，产物在 src-tauri/target/release/bundle/）
npm run tauri:build
```

如果本机使用 MinGW-w64（`%USERPROFILE%\mingw64`）做 gnu 目标构建，可用：

```powershell
pwsh scripts/build.ps1
```

该脚本会先构建前端，再以 `--release` 构建 Rust，并输出生成的 exe 路径。

## 使用说明

1. 启动后程序常驻**系统托盘**，同时打开**设置窗口**。
2. 在设置窗口点「+ 添加小窗」，填写名称与网址。
3. 可选：
   - 录制「显示 / 隐藏快捷键」；
   - 添加「动作」并编写 JS 代码（例如滚动、点击某个按钮），再录制动作快捷键。
4. 点右上角「保存」生效；也可用「显示小窗」按钮立即预览。
5. 在小窗顶部胶囊上按住可拖动窗口；点击胶囊打开下拉栏修改网址或翻页。
6. 关闭设置窗口不会退出程序；退出请用托盘菜单的「退出 GameFloat」。

### 配置文件

设置保存在系统配置目录下的 `gamefloat-config.json`：

- Windows：`%APPDATA%\com.gamefloat.desktop\gamefloat-config.json`

窗口移动 / 缩放只更新内存，**保存、导入或退出时**才写盘。

## 目录结构

```
src/                         前端（Vite root 为 src/renderer）
  shared/types.ts            前后端共享的类型定义
  renderer/
    overlay/index.html       小窗页面入口
    settings/index.html      设置页入口
    src/
      overlay/               Overlay / Pill / Menu + overlay.css
      settings/              Settings + settings.css
      components/            CodeEditorModal、HotkeyInput
      api.ts                 Tauri 命令封装
src-tauri/                   Rust 后端
  src/main.rs                入口、插件注册、命令注册
  src/overlay.rs             小窗、子 webview、下拉栏与动画几何
  src/settings_window.rs     设置窗口
  src/tray.rs                托盘菜单
  src/shortcuts.rs           全局快捷键注册与分发
  src/store.rs               配置读写
  src/model.rs               数据模型
  src/commands.rs            Tauri 命令实现
  capabilities/             权限配置
scripts/                     图标生成、构建脚本
```

## 实现要点

- **一个小窗 = 一个 `Window` + 最多三个子 webview**：内容页、下拉栏、悬浮按钮。子 webview 的层级由创建顺序决定（后创建者在上），所以按「内容 → 下拉栏 → 按钮」创建。
- **按钮尺寸单一来源**：胶囊的宽高只在 Rust 的 `UI_PILL_*` 定义；按钮 webview 即按钮的包围盒，CSS 用 `inset: 0` 填满。菜单层通过 `initialization_script` 拿到同一组尺寸（`window.__GF_PILL__`），据此刻画展开动画的起点，避免 CSS 里重复写死。
- **下拉栏每次打开都新建**：打开时创建对应 webview、关闭时销毁，所以不存在「旧帧」问题，关闭后也能释放内存（代价是每次打开有短暂的创建/加载耗时）。
- **展开动画**：过渡真实尺寸（`top/width/height/border-radius`）而非 `transform: scale`，所以圆角不会被拉伸；透明度与尺寸同时进行。面板高度在首次绘制前量取，保证与内容一致。
- **工具链**：`overlay.rs` 里的 `run_on_main_deferred` 把窗口 / webview 操作统一投递到主线程执行，避免在事件回调中重入死锁。

## 常用脚本

| 命令 | 说明 |
| --- | --- |
| `npm run web:dev` | 只启动 Vite 开发服务器（<http://localhost:5173>） |
| `npm run web:build` | 构建前端到 `dist/` |
| `npm run tauri:dev` | 开发模式启动完整应用 |
| `npm run tauri:build` | 打包发布版（NSIS） |
| `npm run typecheck` | TypeScript 类型检查 |
| `npm run icons` | 纯脚本重新生成应用图标到 `src-tauri/icons/`（无需外部图形库） |
