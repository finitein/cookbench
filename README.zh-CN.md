# Cookbench

<p align="center"><img src="src/assets/cookbench-mark.svg" width="88" height="88" alt="Cookbench 标志"></p>
<p align="center"><a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ja.md">日本語</a> · <a href="README.ko.md">한국어</a></p>

Cookbench 是 Codex、Claude Code、Pi 等编程 Agent 的轻量桌面伴侣。它把每个
被观察到的会话显示为一个小巧的 **Stove**，让你一眼看清哪些任务还在运行、
哪些正在等你、哪些已经完成。原工具始终掌握 control：Cookbench 不托管 Agent，
不发送提示词，不批准工具调用，也不远程控制 Agent。

![Cookbench 按 Harness 分排展示多个 Stove](docs/verification/evidence/e2e-grouped-benches.png)

## 它解决什么问题

- 在一个紧凑的 Bar 中同时查看多个 Harness 的 Session。
- 定位信息可靠时，精准返回终端、IDE 或 Codex Desktop；不足时明确降级。
- Bar 可移动、自由缩放；任务多时自动分排，全量展示且没有滚动条。
- Cooked 由用户清除；可固定长期任务，并从 Archive 恢复过期或误删会话。
- 可选声音、系统横幅、Bar 闪烁和系统提醒。完成闪烁持续到点击对应 Stove。
- SSH 同时支持零安装只读模式和仅使用 SSH stdio、不开端口的单文件 bridge。

## 27 个 Harness Profile

Cookbench 用 **Full**、**Standard**、**Experimental** 三档描述 27 个工具的
身份、生命周期与返回能力，不用一个含糊的“全部支持”掩盖差异。

| 级别 | 工具 |
| --- | --- |
| Full（14） | Codex、Claude Code、Pi、Gemini CLI、Qwen Code、Kimi Code CLI、Qoder、ZCode、Factory Droid、CodeBuddy、Cursor、GitHub Copilot CLI、OpenCode、Cline |
| Standard（12） | Trae、Grok CLI、Goose、Aider、Kiro、Amazon Q Developer、Roo Code、Continue、Amp、Mistral Vibe、Crush、OpenHands CLI |
| Experimental（1） | 腾讯 WorkBuddy，目前仅 presence-only |

Codex、Claude Code、Pi、Kimi Code、ZCode 已支持自动维护 Cookbench 自有 Hook。
其他结构化工具会在 Hook Health 中如实显示手动接入状态。完整边界见
[兼容性矩阵](docs/harness-compatibility.md)。

## 一行命令安装预览版

```bash
# macOS 通用版 / 图形化 Ubuntu Linux x86_64
curl -fsSL https://github.com/finitein/cookbench/releases/download/v0.2.2/install.sh | COOKBENCH_VERSION=v0.2.2 COOKBENCH_ALLOW_PRERELEASE=1 bash
```

```powershell
# Windows x64 PowerShell
$env:COOKBENCH_VERSION='v0.2.2'; $env:COOKBENCH_ALLOW_PRERELEASE='1'; irm https://github.com/finitein/cookbench/releases/download/v0.2.2/install.ps1 | iex
```

脚本会从 `release-manifest.json` 选择原生安装包并先验证 SHA-256。预览包可能
未签名，稳定版和源码构建说明见[安装文档](docs/installing.md)。

## 本地优先与安全边界

原生 Session 文件始终是事实来源。Cookbench 只保存有限的展示元数据、设置、
固定/归档状态和最小跳转信息。它没有 SQLite 对话数据库，不复制完整会话，
不收集遥测，也没有接收消息或远程 control Agent 的接口。

Session roots 留空即可自动扫描当前版本支持工具的标准目录。鼠标悬浮详情默认
关闭，本地通知默认只开启声音，临时报错会在 20 秒后消失。

继续阅读[隐私说明](docs/privacy.md)、[安全边界](docs/security.md)、
[12 张展示图与 HTML 源文件](docs/showcase/README.md)以及
[发布核对表](docs/verification/release-checklist.md)。

Cookbench 是采用 [MIT](LICENSE) 许可证的开源预览版。macOS 与 Ubuntu X11
已有原生验证证据；Windows 和 GNOME Wayland 的真实验证缺口会明确保留。
