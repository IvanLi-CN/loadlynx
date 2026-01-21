# CLI 契约：Web/Storybook 端口行为

本文件定义与端口相关的 `web/` CLI 命令行为（脚本口径），用于保证“端口固定、失败可见、可显式覆盖”。

## 命令清单

> 具体实现方式（用 Vite config 读取 env、或在脚本中传 `--port`）由实现阶段确定；本契约只约束外部可见行为。

| Command | 端口来源 | 预期监听地址 | 端口占用时行为 |
| --- | --- | --- | --- |
| `bun run dev` | `LOADLYNX_WEB_DEV_PORT`（默认见 `contracts/config.md`） | `http://localhost:<port>/` | 退出失败（非零退出码），不得自动换端口 |
| `bun run preview` | `LOADLYNX_WEB_PREVIEW_PORT` | `http://localhost:<port>/` | 退出失败（非零退出码），不得自动换端口 |
| `bun run storybook` | `LOADLYNX_STORYBOOK_PORT` | `http://localhost:<port>/` | 退出失败（非零退出码），不得自动换端口 |
| `bun run test:storybook:ci` | `LOADLYNX_STORYBOOK_TEST_PORT` | `http://127.0.0.1:<port>/`（静态站点服务） | 退出失败（非零退出码），不得自动换端口 |
| `bun run test:e2e` | `LOADLYNX_WEB_DEV_PORT`（间接影响 Playwright baseURL/webServer.url） | N/A | 若 dev server 无法在端口上启动，则测试失败 |

## 环境变量覆盖示例

- 临时改 Web dev 端口：`LOADLYNX_WEB_DEV_PORT=39999 bun run dev`
- 临时改 Storybook 端口：`LOADLYNX_STORYBOOK_PORT=39998 bun run storybook`

## 退出码与错误信息

- 端口占用：必须以非零退出码退出，并在 stderr/stdout 中包含：
  - 端口号
  - 变量名（若该端口来自 env override）
  - 关键字建议包含 `EADDRINUSE` 或同等清晰表述（便于 CI 过滤）

