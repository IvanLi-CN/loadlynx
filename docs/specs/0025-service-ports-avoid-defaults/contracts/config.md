# Config 契约：端口环境变量

本文件定义本计划引入的端口相关环境变量（Config 接口契约）。这些变量用于**显式覆盖**各服务的默认端口。

## 约定

- 变量值类型：十进制整数。
- 允许范围：`1024..=65535`（`0`、负数、非数字均视为非法）。
- 解析失败策略：启动阶段直接失败（非零退出码 + 明确报错）；不得静默回退到默认端口。

## Port registry（默认值）

| Env var | 作用域 | 默认值 | 影响对象 |
| --- | --- | ---: | --- |
| `LOADLYNX_WEB_DEV_PORT` | local dev / CI | 25219 | `web/` Vite dev server（以及 Playwright `baseURL/webServer.url`） |
| `LOADLYNX_WEB_PREVIEW_PORT` | local dev / CI | 22848 | `web/` Vite preview server |
| `LOADLYNX_STORYBOOK_PORT` | local dev | 32931 | `web/` Storybook dev server |
| `LOADLYNX_STORYBOOK_TEST_PORT` | CI / local | 34033 | Storybook 静态站点测试服务器（`http-server`） |

## 行为要求（必须满足）

- Vite dev server / preview server 必须启用“严格端口”（端口占用则失败），不得自动 +1 或随机选端口。
- Storybook dev server 必须启用“精确端口”（端口占用则失败），不得自动换端口。
- Storybook 测试静态站点服务器必须使用固定端口，禁止 `get-port` 等动态选端口方案。
