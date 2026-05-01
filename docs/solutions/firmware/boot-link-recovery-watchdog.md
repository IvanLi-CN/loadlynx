# Boot link recovery watchdog 模式

## 适用场景

- 两个 MCU 上电时序不完全可控，任一侧可能先完成 UART 初始化、先发送 HELLO 或先进入 quiet window。
- UI 渲染任务本身仍在运行，但遥测从未收到第一帧，导致屏幕看起来像“活着的冻结状态”。
- 控制发送任务已经是 UART TX 的单 owner，能串行化 SoftReset、校准同步和当前控制 snapshot。

## 结论

- **生产初始 UI 不应使用 demo telemetry。** Demo 快照只能用于 mock/test；真实固件启动时必须显示 offline/unknown，直到收到第一帧真实遥测。
- **恢复 watchdog 要覆盖 `LAST_GOOD_FRAME_MS==0`。** 只在 `LINK_UP=true` 之后处理 CalMissing 不够；冷启动最危险的状态是“从未见过任何有效帧”。
- **链路有效不等于测量有效。** 冷上电后收到 `HELLO`、ACK、没有后续 `FastStatus`，或只收到全零 `FastStatus`，都只能证明协议路径在动；必须等到 `FastStatus` 出现非零电压、电流或功率信号，才把 `0V/0A/0W` 视作可信读数。
- **完整恢复包要由 UART TX 单 owner 发出。** SoftReset、全量 CalWrite、SetEnable、LimitProfile 与 forced SetMode snapshot 必须走同一个发送任务，避免多个 task 并发写串口造成帧交错。
- **SoftReset ACK 不能用单个陈旧 boolean。** 每次握手都应记录发送前 ACK 计数和本次 seq，只有“计数增长且 seq 匹配”才算本次 ACK 到达。
- **恢复必须限频且不绕过安全 gate。** 模拟板未上电、隔离器异常或线缆断开时，watchdog 只能低频重试与记录日志；不能为了恢复而盲发 output-enabled=true。

## 推荐实现模式

1. Telemetry model 初始化为 offline snapshot。
2. UART RX 每收到有效 `HELLO` / `FAST_STATUS` / ACK 都刷新 `LAST_GOOD_FRAME_MS`。
3. UART TX owner 周期检查：
   - `LAST_GOOD_FRAME_MS == 0` 且超过 boot grace；或
   - 曾经有帧但 `LINK_UP=false` 持续超过 stale grace；或
   - 已经有帧但没有 `FastStatus`，或还没见过可信非零测量，且超过 measurement grace。
4. 若当前没有 pending 控制 ACK，按低频 retry window 发起恢复：
   - SoftReset handshake（seq/baseline 绑定 ACK）
   - quiet gap
   - 全量 CalWrite
   - SetEnable(true)
   - LimitProfile
   - 标记下一轮必须发送当前 SetMode snapshot
5. output-on 命令仍遵守现有 link/fault/offline gate。

## 本仓库落点

- 数字板 UART TX owner：`firmware/digital/src/main.rs`
- Dashboard snapshot：`firmware/digital/src/ui/mod.rs`
- 相关规格：`docs/specs/fqmns-boot-link-recovery/SPEC.md`

## 为什么这套模式有效

- 它把“UI 是否活着”和“遥测是否曾经建立”拆成两个状态，避免 demo 值掩盖链路失败。
- 它让恢复动作由已有 UART TX owner 串行发出，降低帧交错和竞态风险。
- 它用 seq/baseline 消除陈旧 ACK，保证每次恢复尝试都有可验证的握手边界。
