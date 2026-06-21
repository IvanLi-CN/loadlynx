# LoadLynx 电脑安装与更新

这篇指南面向普通用户机器。

- `程序` 指 GitHub Releases 发布的 `loadlynx` 和 `loadlynx-devd`。
- `skill` 指 `loadlynx-user-operations`。
- 程序安装和更新使用官方 Release installer 与 `SHA256SUMS`。
- skill 安装和更新使用官方 `npx skills` 命令。

## 1. 安装或更新程序

### macOS / Linux

安装最新稳定版：

```bash
curl -fsSL https://github.com/IvanLi-CN/loadlynx/releases/latest/download/install-loadlynx-host.sh \
  | bash
```

强制重装或升级到同版本：

```bash
curl -fsSL https://github.com/IvanLi-CN/loadlynx/releases/latest/download/install-loadlynx-host.sh \
  | bash -s -- --force
```

安装或更新到指定 tag：

```bash
curl -fsSL https://github.com/IvanLi-CN/loadlynx/releases/latest/download/install-loadlynx-host.sh \
  | bash -s -- --version <tag> --force
```

也可以先下载脚本再执行：

```bash
bash install-loadlynx-host.sh --version <tag> --force
```

### Windows PowerShell

安装最新稳定版：

```powershell
iwr https://github.com/IvanLi-CN/loadlynx/releases/latest/download/install-loadlynx-host.ps1 -OutFile install-loadlynx-host.ps1
powershell -ExecutionPolicy Bypass -File .\install-loadlynx-host.ps1
```

强制重装或升级到同版本：

```powershell
powershell -ExecutionPolicy Bypass -File .\install-loadlynx-host.ps1 -Force
```

安装或更新到指定 tag：

```powershell
powershell -ExecutionPolicy Bypass -File .\install-loadlynx-host.ps1 -Version <tag> -Force
```

### 手动下载 archive

如果 installer 无法联网或下载失败，可以手动从 GitHub Releases 下载对应平台的 host-tools archive。

必须先校验该 archive 对应的 release `SHA256SUMS`，再解压安装。不要跳过这一步。

常见平台文件名：

- Apple Silicon macOS: `loadlynx-host-tools-macos-aarch64.tar.gz`
- Intel macOS: `loadlynx-host-tools-macos-x86_64.tar.gz`
- Linux x86_64: `loadlynx-host-tools-linux-x86_64.tar.gz`
- Windows x86_64: `loadlynx-host-tools-windows-x86_64.tar.gz`

## 2. 安装或更新 skill

全局安装 `loadlynx-user-operations`：

```bash
npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-user-operations -g
```

全局更新这个 skill：

```bash
npx skills update loadlynx-user-operations -g
```

这里的 `-g` 表示把 skill 装到这台电脑的全局 skill 目录，而不是当前项目目录。

## 3. 安装后验证

先验证程序：

```bash
loadlynx -v
loadlynx --help
loadlynx-devd --help
```

再验证 skill：

```bash
npx skills list -g
```

你应该能看到 `loadlynx-user-operations`，并且 `loadlynx --help` / `loadlynx-devd --help` 都能正常输出。

## 4. 何时用更新

- 程序更新：重跑官方 installer；需要覆盖已有安装时使用 `--force` 或 `-Force`。
- skill 更新：使用 `npx skills update loadlynx-user-operations -g`。
- 如果要固定到某个已发布版本，程序 installer 使用 `--version <tag>` 或 `-Version <tag>`。

## 5. 相关入口

- 普通用户硬件操作说明：[`skills/loadlynx-user-operations/SKILL.md`](../../skills/loadlynx-user-operations/SKILL.md)
- 项目主页：[`README.md`](../../README.md)
