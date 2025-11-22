# TODO

- [x] 修复数字板在启用 `mock_setpoint` 时启动 panic：报错 “Clocks have not been initialized yet”。
      复现：`FEATURES=mock_setpoint scripts/agent_verify_digital.sh --timeout 20`（单板）或 dual monitor。
      验收：启用 mock_setpoint 时数字固件能正常启动并运行 setpoint 发送任务，无 panic。
      说明：旧 dual-monitor 日志中的 panic 出现在早期版本（日志里版本号为“14”）；在当前 HEAD=8254578 上按上述命令复测时，该 panic 未再出现，mock_setpoint 与 setpoint 任务均正常运行，因此可以确认当前版本中不存在这一问题。
- [ ] 确认 SetPoint/ACK 链路在修复后无丢包。
      复现：通过 mock_setpoint 或旋钮产生 setpoint，dual monitor 40 秒。
      验收：数字日志出现 `setpoint sent`，模拟日志出现 `SetPoint received`，decode_err=0。
- [ ] 消除正常运行下的协议解码错误（payload length mismatch）。
      复现：默认设置 dual monitor 40 秒。
      验收：decode_err=0，fast_status_ok 连续递增。
