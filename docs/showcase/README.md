# Cookbench 中文展示图

本目录包含 14 张离线生成的 1200×1500 中文 HTML 宣传页，以及对应的确定性
PNG 渲染结果。它们可用于项目公告、社交平台发布、社区介绍和视频封面或插图。
内容强调 Cookbench 的超轻量桌面体验、完整开源代码、可 DIY 的适配边界，以及
可在系统语言、简体中文、English、日本語、한국어之间切换的界面。

| 页面 | 受众与主题 |
| --- | --- |
| [01 项目概览](01-overview.html) | AI 初学者：超轻量、开源、DIY 与多语言的 Cookbench |
| [02 一眼看清](02-one-glance.html) | 同时运行多个 Agent 会话的人：轻量总览 |
| [03 适配目录](03-catalog.html) | 27 个全球及中国本土 Harness Profile，支持 DIY 扩展 |
| [04 支持分级](04-tiers.html) | 诚实说明完整、标准、实验性能力 |
| [05 精准跳转](05-return.html) | 会话到窗口的强身份链与精准返回 |
| [06 跨平台](06-platforms.html) | macOS、Windows、图形化 Linux 与中英日韩界面 |
| [07 SSH](07-ssh.html) | 零安装只读模式与可选 stdio Bridge |
| [08 隐私](08-privacy.html) | 本地优先、开源可审查的元数据边界 |
| [09 Hook](09-hooks.html) | Hook 安装、健康、修复、内容过滤与 DIY |
| [10 任务管理](10-workflow.html) | 通知、固定、归档、完成确认与多语言设置 |
| [11 多排工作台](11-multibench.html) | 无滚动条的重度多 Agent 布局 |
| [12 一行安装](12-install.html) | 一行命令安装、开源、DIY 与中英日韩支持 |
| [13 资源占用](13-footprint.html) | macOS 实测的低内存、低存储占用与轻量架构原因 |
| [14 聚焦界面](14-focus-surfaces.html) | 极简模式、顶端吸附与 macOS 合并状态栏 Stove |

所有页面仅使用 Cookbench 自有 SVG 标志、系统字体、CSS 和本地产品证据；不含
第三方 Logo、远程资源、图库照片、字体包、GIF、视频、Lottie 或复制的产品图稿。

第 14 页额外导出 `rendered/14-focus-surfaces-social.png`，为小红书和抖音保留
1080×1440 的竖版安全区：关键信息在 x=72..1008、y=120..1320 内。它和 1200×1500
主图来自同一个本地 HTML，不包含截图式的原生功能证明。

重新渲染已提交的 PNG：

```bash
pnpm showcase:render
```
