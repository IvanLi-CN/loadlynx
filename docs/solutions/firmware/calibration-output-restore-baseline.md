# Calibration 输出 override 要保持单独的“恢复基线”

## 适用场景

- 数字板在正常 preset 输出之外，还提供 current calibration 的临时 CC override。
- 同一个 `output_enabled` 既被 UI/LED/屏保等本地状态消费，也会被最终的 `SetMode` 下发链消费。
- 主人可以在 calibration 期间通过 Web API 或实体按键反复开关输出，但退出 calibration 后仍希望恢复到进入 calibration 前的正常模式状态。

## 结论

- **current calibration 入口必须同步 live output flag。** 一旦从正常模式切入 `current_ch1/current_ch2`，`ControlState.output_enabled` 就必须立刻对齐当前 calibration override 的有效输出状态；否则 UI 会把“正常模式下的 ON”误认为“校准模式下仍在输出”。
- **只有收到 analog 的 `CAL_MODE` ACK 后，才允许切换数字侧的 calibration mode 状态。** 先把 `cal_mode`/live output 改掉，会让数字侧 `SetMode` 目标和真实硬件模式脱节；校准模式切换必须以 ACK 作为生效边界。
- **ACK 生效顺序要先收口 control state，再发布新的 `cal_mode`。** `setmode_tx_task()` 会分两把锁读取 calibration mode 和 `ControlState`；如果先发布新 mode、后清理 calibration override，就会短暂把正常 preset 和校准态遗留的 `output_enabled=true` 拼在一起，下发错误的瞬时 `SetMode`。
- **进入 calibration 时要先捕获 restore baseline。** 这个 baseline 表示“离开 calibration 后正常 preset 应恢复到什么输出状态”，它和 calibration 期间的临时 ON/OFF 不是同一个概念。
- **calibration 期间的开关只能改 override，不应覆盖 restore baseline。** 无论是 `/api/v1/control`、`/api/v1/cc`，还是实体 LOAD 开关，在 current calibration 内都应只改变 override 的 `output_enabled`，不能把“退出 calibration 时恢复什么”改写掉。
- **`current_ch1` / `current_ch2` 之间切换不能顺手清掉 override。** 只要还留在 current calibration 家族里，临时 CC target 与 calibration 期间的输出状态都应被保留；真正清理 override 的时机只有“离开 current calibration”。
- **calibration override 不能绕过 active preset 的保护线。** `SetMode` 期间看到的 effective preset 可以改成临时 CC target，但仍必须保留当前 active preset 的 `min_v_mv / max_i_ma_total / max_p_mw`；否则校准态会静默丢掉 UVLO/OCP/OPP。
- **真正需要清空 restore baseline 的场景应是保护性关断。** 例如 UVLO、OCP/OPP、链路保护等强制 OFF，可以视为正常模式也不该自动恢复；用户在 calibration 内的普通开关则不属于这种场景。

## 推荐实现模式

1. 进入 `current_ch1/current_ch2` 时：
   - 若 restore baseline 尚未记录，先记录当前 `output_enabled`
   - 再把 live `output_enabled` 同步到 calibration override 的有效状态
2. current calibration 内的用户开关：
   - `enable=true`：要求已有非零 calibration target，再只更新 override 的 `output_enabled`
   - `enable=false`：只关闭 override/live output，不动 restore baseline
3. 离开 current calibration 时：
   - 先完成 control-side 的 override / restore baseline 收口，再发布新的 `cal_mode`
   - 清掉 calibration override
   - 按 restore baseline 恢复正常模式输出状态
   - 若恢复前被安全门禁否决，再显式把 baseline 调整成 `false`

## 本仓库落点

- 控制状态：`/Users/ivan/Projects/Ivan/loadlynx/firmware/digital/src/control.rs`
- Calibration mode 切换：`/Users/ivan/Projects/Ivan/loadlynx/firmware/digital/src/net.rs`
- 实体 LOAD 开关：`/Users/ivan/Projects/Ivan/loadlynx/firmware/digital/src/main.rs`

## 为什么这套模式有效

- 它把“当前 calibration 是否真的在输出”和“退出 calibration 后正常模式该恢复成什么”拆成了两个状态层次。
- 这样可以同时保证：
  - 当前屏幕/LED/SetMode 命令看到的是**真实生效的 calibration 输出状态**
  - 退出 calibration 时恢复的是**进入 calibration 前的正常模式状态**
- 只要保护性关断仍走单独的强制 OFF 路径，就不会把安全语义和普通校准开关语义混在一起。
