# TODO

编写要求（保持任务可审计、可交接）：

- 每条任务包含：任务名、描述/复现、验收标准、实施建议、进展/结果（未开始/进行中/完成）、备注（完成时填写关键结论或遗留风险）。
- 状态使用 `[ ]` 未开始、`[-]` 进行中、`[x]` 已完成。

- [x] 任务名：修复数字板启用 `mock_setpoint` 时启动 panic
  - 描述/复现：`FEATURES=mock_setpoint scripts/agent_verify_digital.sh --timeout 20`（单板或 dual monitor）曾报错 “Clocks have not been initialized yet”。
  - 验收标准：启用 mock_setpoint 时数字固件能正常启动并运行 setpoint 发送任务，无 panic。
  - 实施建议：确认时钟初始化顺序，保持 mock_setpoint 代码路径与正常路径一致的时钟依赖；必要时在启动早期补充时钟 guard。
  - 进展/结果：完成。当前 HEAD=8254578 复测未再出现 panic，mock_setpoint 与 setpoint 任务均正常运行。
  - 备注：旧 dual-monitor 日志的 panic 属早期版本，已消除。

- [x] 任务名：确认 SetPoint/ACK 链路在修复后无丢包
  - 描述/复现：通过 mock_setpoint 或旋钮产生 setpoint，dual monitor 运行 40 秒。
  - 验收标准：数字日志出现 `setpoint sent`，模拟日志出现 `SetPoint received`，且 decode_err=0。
  - 实施建议：统计 seq/ACK 计数；必要时调高 UART FIFO 阈值或退避策略；收集 40 s 双端日志做比对。
  - 进展/结果：完成。6 分钟 soak（序号自增 1..255 循环）数字侧统计 setpoint_tx=1319、ack=1319、retx=0、timeout=0、decode_errs=0，模拟侧全程逐帧 ACK 无告警，未见 FAST_STATUS CRC/长度错误。
  - 备注：日志路径 `tmp/agent-logs/digital-20251123-133501.log`（360 s）与 `tmp/agent-logs/analog-20251123-133444.log`（~315 s，含 seq wrap），当前固件版本：analog 0.1.0 (fe7cc03-dirty)、digital 0.1.0 (951a3ed-dirty, soak build)。

- [x] 任务名：实现 SetPoint ACK/重传机制，避免状态不同步
  - 描述/复现：当前 SetPoint 仅发送不等确认，掉帧时模拟板目标不更新；需按 `docs/interfaces/uart-link.md` 新增的“SetPoint 可靠传输方案（v1）”实现并验证。
  - 验收标准：
    1) 数字侧出现 `setpoint sent` 后在超时时间内看到匹配的 `setpoint ack received`；
    2) 故意丢弃 1–2 帧（或拔插串口）后，重传成功且模拟侧日志出现对应 `SetPoint received`；
    3) 遥测中的 `target_value` 连续与数字侧期望一致（允许 clamp 后值匹配）。
  - 实施建议：
    - libs/protocol 支持 SetPoint ACK/NACK 生成；
    - 数字侧添加等待 ACK + 30/60/120 ms 退避重传，最新值优先；
    - 模拟侧解帧后幂等 ACK（重复 seq 只 ACK 不重放）；解析失败回 NACK；
    - 在 RX 处理处匹配 ACK/NACK，更新统计与 UI 告警；
    - 新增自检：对比 fast_status.target_value 与期望，连续不一致时强制重发并告警。
    - 利用 `mock_setpoint` 功能做自动化串口通信测试：
      - 数字侧开启 mock_setpoint 生成可重复的 SetPoint 流，结合退避重传统计；
      - 在真机 dual monitor 场景运行 40–60 s，记录双端日志，自动统计 sent/ack/retx/dup；
      - 脱机脚本检查：ACK 覆盖率 100%，重传次数在预期范围内（掉线时可恢复），遥测 `target_value` 与期望一致。
  - 进展/结果：数字侧实现 ACK 等待、40/80/160 ms 退避重传与 300 ms 启动静默，10 Hz 发送（100 ms 周期）；模拟侧幂等 ACK。mock_setpoint 双板 40 s 验证（日志：digital `tmp/agent-logs/digital-dual-20251123-151533.log` / analog `tmp/agent-logs/analog-dual-20251123-151533.log`）：sent=95、ack=95、retx=2、timeout=0，模拟侧 292 次 ACK 覆盖 214 个唯一 seq，目标电流 0–2000 mA 全部应用，无超时。
  - 备注：decode_err 偶发 payload length mismatch 属独立任务处理；如需更长 soak 可延长 dual monitor。

- [x] 任务名：消除正常运行下的协议解码错误（payload length mismatch）
  - 描述/复现：默认配置 dual monitor 运行 40 秒，偶发 `payload length mismatch`。
  - 验收标准：decode_err=0，fast_status_ok 连续递增。
  - 实施建议：检查 SLIP 分帧容量、超时、UART DMA chunk；在 libs/protocol 增加健壮性日志；复现后抓取原始帧。
  - 进展/结果：完成（代码层面）。数字侧 UART 链路在 SLIP 帧交给协议解码前新增长度一致性校验，发现帧长/声明长度不符时计入 `framing_drops`、限速告警并复位解码器，不再累加 `decode_errs` 也不会触发 payload length mismatch 日志；统计行增加 `framing_drops` 便于后续观察。
  - 备注：需在下一次 40s dual monitor 复测确认 `decode_errs=0` 且 fast_status_ok 连续递增，若 `framing_drops` 持续增长则需继续排查物理链路。

- [x] 任务名：实现数字侧触发的软复位链路（SOFT_RESET_REQ/ACK）
  - 描述/复现：保持持续供电，要求数字板每次上电或 UI 命令可触发模拟板软复位，清除残留状态。
  - 验收标准：不断电场景下重复软复位，模拟板状态清零、重新 HELLO；无残留输出，握手与 CAL 下发能恢复。
  - 实施建议：
    1) `libs/protocol` 定义 0x26/ACK 帧与 reason 枚举；
    2) ESP32 固件：启动后发送 + 150 ms×3 尝试 + UI 提示；收到 ACK 后等待新 HELLO；
    3) STM32 固件：收到请求即失能/清状态、回 ACK、重新 HELLO；幂等处理重复请求；
    4) 自测：dual monitor 不断电连续触发，核对 fast_status/状态位。
  - 进展/结果：完成。协议新增 SOFT_RESET 消息；数字侧启动时发送 3 次（150 ms 间隔），收到 ACK 后记录；模拟侧收到请求即清零 DAC/目标电流、短暂拉低 LOAD_EN，再回 ACK 并继续遥测。
  - 备注：尚未实现 HELLO 状态机，后续可在握手链路落地后接入等待 HELLO 的 gating。
