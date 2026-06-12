# Web production preview smoke 与 chunk-cycle regression History

- 2026-06-13: 将 `https://loadlynx.ivanli.cc/` 的白屏事故正式归类为 production bundle runtime crash，而非“页面慢”；新增 production preview smoke 与 CI/deploy 前置门禁，并取消独立 `react-vendor` manual chunk 以打破循环初始化。

## Documentation Model

`SPEC.md` 维护当前 topic contract；关键演进原因与事故定性保留在这里。
