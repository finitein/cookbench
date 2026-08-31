# Cookbench

<p align="center"><img src="src/assets/cookbench-mark.svg" width="88" height="88" alt="Cookbench ロゴ"></p>
<p align="center"><a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ja.md">日本語</a> · <a href="README.ko.md">한국어</a></p>

Cookbench は、普段使っているコーディング Agent のための軽量デスクトップ
コンパニオンです。各セッションを小さな **Stove** として並べ、実行中、確認待ち、
完了を一目で把握できます。元のツールが常に control を持ち、Cookbench は Agent
をホストせず、プロンプト送信、ツール承認、遠隔操作を行いません。

![Harness ごとに並ぶ Cookbench の Stove](docs/verification/evidence/e2e-grouped-benches.png)

## 主な機能

- 複数 Harness の Session をコンパクトな Bar でまとめて確認。
- 識別情報を検証できた場合のみ、元のターミナル、IDE、Codex Desktop へ正確に復帰。
- Bar は移動と自由なサイズ変更に対応。Stove が多い場合は自動で段組みし、
  スクロールバーなしで全件表示。
- Cooked は手動で消すまで保持。長期タスクのピン留めと Archive からの復元。
- サウンド、システムバナー、Bar 点滅、注意要求を選択可能。完了点滅はクリックまで継続。
- SSH はリモート無導入の読み取り専用方式と、SSH stdio だけを使う bridge に対応。

## 27 の Harness Profile

**Full**、**Standard**、**Experimental** の 3 段階で 27 ツールの識別、
ライフサイクル、復帰能力を表し、すべてが同じ機能だとは主張しません。

| Tier | 対象 |
| --- | --- |
| Full（14） | Codex、Claude Code、Pi、Gemini CLI、Qwen Code、Kimi Code CLI、Qoder、ZCode、Factory Droid、CodeBuddy、Cursor、GitHub Copilot CLI、OpenCode、Cline |
| Standard（12） | Trae、Grok CLI、Goose、Aider、Kiro、Amazon Q Developer、Roo Code、Continue、Amp、Mistral Vibe、Crush、OpenHands CLI |
| Experimental（1） | Tencent WorkBuddy、現在は presence-only |

Codex、Claude Code、Pi、Kimi Code、ZCode は Cookbench 所有 Hook の自動設定に
対応します。それ以外は Hook Health に手動接続として表示します。詳細は
[互換性マトリクス](docs/harness-compatibility.md)を参照してください。

## 1 コマンドでプレビュー版を導入

```bash
# macOS universal / GUI Ubuntu Linux x86_64
curl -fsSL https://github.com/finitein/cookbench/releases/download/v0.2.1/install.sh | COOKBENCH_VERSION=v0.2.1 COOKBENCH_ALLOW_PRERELEASE=1 bash
```

```powershell
# Windows x64 PowerShell
$env:COOKBENCH_VERSION='v0.2.1'; $env:COOKBENCH_ALLOW_PRERELEASE='1'; irm https://github.com/finitein/cookbench/releases/download/v0.2.1/install.ps1 | iex
```

`release-manifest.json` からネイティブパッケージを選び、SHA-256 検証後にだけ
インストールします。プレビューは未署名の場合があります。詳細は
[インストールガイド](docs/installing.md)にあります。

## ローカル優先

ネイティブ Session ファイルが事実源です。限定された表示メタデータ、設定、
ピン/Archive、最小限の復帰 locator だけを保存します。SQLite 会話 DB、全文コピー、
テレメトリ、受信メッセージ、遠隔 control API はありません。

Session roots を空欄にすると標準ディレクトリを自動走査します。ホバー詳細は
既定でオフ、通知はサウンドだけがオン、一時エラーは 20 秒後に消えます。

[Privacy](docs/privacy.md)、[Security](docs/security.md)、
[12 枚の紹介画像と HTML](docs/showcase/README.md)、
[リリースチェックリスト](docs/verification/release-checklist.md)も参照してください。

[MIT License](LICENSE) のオープンソースプレビューです。macOS と Ubuntu X11
には実機証拠があり、Windows と GNOME Wayland の未検証項目は明記します。
