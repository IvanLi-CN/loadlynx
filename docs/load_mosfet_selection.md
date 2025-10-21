# 电子负载 N 沟道 MOSFET 选型（线性模式、散热≤100 °C）

## 选型要点（必须同时满足）

1. **线性工作余量**：单管承受 1.8 V–48 V 输入、100 W 连续、200 W（短时）；需要足够大的 DC / 脉冲 SOA。  
2. **低压大电流能力**：当前设计假设回路中存在 25 mΩ 采样电阻、最大约 0.8 V 防反二极管压降及一次性保险丝（按 0.1–0.2 V 估计）；MOSFET 自身的 10 A 掉压越小，越能在 1.8 V 系统底端维持设定电流。硬件定稿后应以原理图/BOM 实测值替换本假设。  
3. **OPA2365AIDR 栅驱兼容性**：运放在 5 V 供电时能提供约 ±65 mA，栅极所需 ΔV≈I/g<sub>fs</sub>。高 g<sub>fs</sub> 和较低 Q<sub>g</sub> 能保证闭环控制不过载。  
4. **散热限制**：AFC0612D 风扇 + 60×60×60 “风洞”散热片的综合换热系数约 30–40 W/m²·K，对应 `hA ≈ 1.98–2.64 W/K`（`docs/fans/fan_heatsink_integration_afc0612d.md:27-58`）。若环境 25 °C 且散热器表面温度需 ≤100 °C，单管稳态可持续功耗需 ≤`hA×(100-25)` ≈ 150–198 W。  
5. **成本与可购性**：仅从 `docs/load_mosfet_candidates.md:5-25` 的淘宝真实在售型号中选择，兼顾价格与供应。

基于上述约束，100 W 连续（48 V×≈2.1 A）能在 25 °C 环境下保持散热器 <100 °C；对 200 W 短时负载，应以“功率积分/移动平均功率”与表面温度双重限幅来约束热积累（时间窗口由热容/热时常实测标定），并通过导风密封优化工作点流量。

## 入围器件对比

| 型号 | 价格 (¥) | V<sub>DS</sub> (V) | R<sub>DS(on)</sub> @10 V | g<sub>fs</sub> (typ) | Q<sub>g</sub> (typ) | R<sub>θJC</sub> (°C/W) | 10 A 掉压 | 备注 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| **IRFP4468PBF** | 5.78 (`docs/load_mosfet_candidates.md:24`) | 100 | 2.6 mΩ (`docs/load-mosfets/IRFP4468PBF.md:82-90`) | 185–370 S (`docs/load-mosfets/IRFP4468PBF.md:82-90`) | 363 nC (`docs/load-mosfets/IRFP4468PBF.md:94-101`) | 0.29 (`docs/load-mosfets/IRFP4468PBF.md:72-74`) | 0.026 V | 主推，低掉压 & 大芯片 |
| **IRFP4568PBF** | 8.62 (`docs/load_mosfet_candidates.md:23`) | 150 | 5.9 mΩ (`docs/load-mosfets/IRFP4568PBF.md:40-48`) | 162 S (`docs/load-mosfets/IRFP4568PBF.md:44-46`) | 151 nC (`docs/load-mosfets/IRFP4568PBF.md:44-54`) | 0.29 (`docs/load-mosfets/IRFP4568PBF.md:33-36`) | 0.059 V | 高耐压 150 V，SOA 裕度佳 |
| **IRFP4668PBF** | 7.99 (`docs/load_mosfet_candidates.md:14`) | 200 | 9.7 mΩ (`docs/load-mosfets/IRFP4668PBF.md:40-46`) | 150 S (`docs/load-mosfets/IRFP4668PBF.md:40-46`) | 161 nC (`docs/load-mosfets/IRFP4668PBF.md:44-53`) | 0.29 (`docs/load-mosfets/IRFP4668PBF.md:30-37`) | 0.097 V | 200 V 方案，适合高压测试线 |
| IRFP4227PBF | 6.86 (`docs/load_mosfet_candidates.md:13`) | 200 | 25 mΩ (`docs/load-mosfets/IRFP4227PBF.md:26-36`) | 49 S (`docs/load-mosfets/IRFP4227PBF.md:26-36`) | 70–98 nC (`docs/load-mosfets/IRFP4227PBF.md:26-36`) | 0.45 (`docs/load-mosfets/IRFP4227PBF.md:22-24`) | 0.25 V | 中压兼顾成本，适合降额应用 |
| IRFP260NPBF | 3.19 (`docs/load_mosfet_candidates.md:20`) | 200 | 40 mΩ (`docs/load-mosfets/IRFP260NPBF.md:7-45`) | 27 S (`docs/load-mosfets/IRFP260NPBF.md:38-45`) | 234 nC (`docs/load-mosfets/IRFP260NPBF.md:42-48`) | 0.50 (`docs/load-mosfets/IRFP260NPBF.md:34-36`) | 0.40 V | 超低成本，但需在 1.8 V 端降额 |

*10 A 掉压 = R<sub>DS(on,max)</sub> × 10 A，用于评估 1.8 V 输入时的电压余量。*

### 综合分析

#### **IRFP4468PBF** —— 主力推荐

- **低掉压**：10 A 仅损耗 26 mV，搭配 25 mΩ 采样电阻（0.25 V）+ 0.8 V 二极管 + ~0.15 V 保险丝，总压降仍 <1.3 V，1.8 V 输入下有 >0.5 V V<sub>DS</sub> 余量。  
- **控制友好**：g<sub>fs</sub> ≥185 S。需注意：数据手册 R<sub>DS(on)</sub> 规格点在 V<sub>GS</sub>=10 V，5 V 栅驱属于“非完全增强”工况，低压大电流能力应以 5 V 驱动的台架实测为准（见“5 V 栅驱能力估算”节）。  
- **散热优势**：R<sub>θJC</sub> 0.29 °C/W，有助于把热流导向散热器。SOA 图（`docs/load-mosfets/IRFP4468PBF_images/e9934be03e1c61ad17c19da6fae0c8cdeac2c49b7c22b4c5d9ec242d3b392065.jpg`）需配合散热器测试确认 200 W 脉冲时的安全时长。

#### **IRFP4568PBF** —— 更高耐压的平衡方案

- 150 V 耐压对瞬态尖峰更稳健；R<sub>DS(on)</sub>≈5.9 mΩ，10 A 掉压 59 mV。  
- R<sub>θJC</sub> 0.29 °C/W，SOA 在 48 V 直流段约 0.6 A，但 1 ms 脉冲可达 7 A（`docs/load-mosfets/IRFP4568PBF_images/352c84452941ad8deaafabe5a39e88bddf0423f40ae037963410c7827a43e639.jpg`）。配合 100 W 连续工作需实测验证散热。  
- Q<sub>g</sub> 仅 151 nC，驱动负担低，适合作为“高耐压 SKU”。

#### **IRFP4668PBF** —— 200 V 高压冗余

- 200 V / 130 A 能覆盖更宽的应用，但 R<sub>DS(on)</sub>≈9.7 mΩ，10 A 掉压提升到 0.097 V。  
- g<sub>fs</sub> 约 150 S，OPA2365 仍可直接驱动；SOA 1 ms 曲线 ≈5 A（`docs/load-mosfets/IRFP4668PBF_images/5cd29ceb9f5cb0cdaa85477b063275f52590c2d7bfce4fd6e3dcdf3b86c8b31b.jpg`），适合在高压实验线中配合降额使用。  
- 成本略高于 **IRFP4468PBF**，可作为“耐压冗余”选项。

#### IRFP4227PBF —— 中等成本的 200 V 器件

- R<sub>DS(on)</sub> 21–25 mΩ，10 A 掉压约 0.25 V。结合采样电阻和防反二极管之后，1.8 V 模式下只剩约 0.45–0.55 V 的 V<sub>DS</sub>，因此需将大电流限制在 ≥2.2 V 输入。  
- g<sub>fs</sub> 仅 49 S，意味着同样的电流需要更高的栅极增量，对 OPA2365AIDR 的 5 V 输出逼近极限。  
- 优点是 200 V 耐压与 6.86 ¥ 的适中价格，适合对成本敏感、允许低压端降额的版本。

#### IRFP260NPBF —— 低成本备选

- 仅 3.19 ¥，但 R<sub>DS(on)</sub>=40 mΩ、g<sub>fs</sub>=27 S，10 A 掉压 0.40 V。  
- 当输入压 1.8 V 时，扣除其他器件（约 1.15 V）后仅剩 ≈0.25 V 的调节余量， **必须限制低压端电流≤8 A 或增加输入裕量**。  
- Q<sub>g</sub>=234 nC、R<sub>θJC</sub>=0.50 °C/W，也使驱动/散热压力更大。因此只建议在成本极敏感且功率降额的场合选用。

## 热设计与功率限制

- 目标：散热器表面温度 ≤100 °C。保守取 `hA = 1.98 W/K`（`docs/fans/fan_heatsink_integration_afc0612d.md:27-58`），环境温度 25 °C。  
- 稳态功耗上限：`P_cont ≈ hA × (100-25) = 1.98 × 75 ≈ 148 W`（保守口径）。  
- 48 V 工作时，对应 **连续电流 ≈3.1 A**，即 100 W 连续负载（≈2.1 A）安全；**短时 200 W** 需采用“功率积分（移动窗口）+ 表面温度”双限幅控制以限制热积累。若通过导风密封把 `hA` 推近 2.64 W/K，200 W 工况的稳态 ΔT≈76 K，但仍需以实测热时常/热容标定时间窗口。  
- 建议在散热器上贴热敏元件/热电偶，与固件功率积分联动，确保实际温升满足约束。

## 5 V 栅驱能力（理论估算）

为评估 5 V 驱动下的线性导通能力，采用一阶三极管区近似：在欧姆区内有
`R_ds,on ∝ 1 / (V_ov) ≈ 1 / (V_gs − V_th)`。
据此由数据手册的 `R_ds,on@10 V` 与 `V_th`（门限，V<sub>GS(th)</sub>）估算 5 V 栅驱时的等效电阻：

`R5 ≈ R10 × (10 − V_th) / (5 − V_th)`（当 `5 ≤ V_th` 时无解，视为无法可靠完全导通）。

阈值电压区间来自各器件数据手册：
- **IRFP4468PBF**：V<sub>GS(th)</sub>=2.0–4.0 V（`docs/load-mosfets/IRFP4468PBF.md:82-90`），且门极平台电压 V<sub>plateau</sub>≈4.8 V（`docs/load-mosfets/IRFP4468PBF.md:94-106`）。
- **IRFP4568PBF**：V<sub>GS(th)</sub>=3.0–5.0 V（Infineon 官方 DS）。
- **IRFP4668PBF**：V<sub>GS(th)</sub>=3.0–5.0 V（Infineon 官方 DS）。
- IRFP4227PBF：V<sub>GS(th)</sub>=3.0–5.0 V（Infineon 官方 DS）。
- IRFP260NPBF：V<sub>GS(th)</sub>=2.0–4.0 V（Infineon/Vishay DS）。

在当前“低压端串联压降假设”（采样 0.25 V + 防反 0.8 V + 保险丝 0.15 V，总计≈1.20 V）下，1.8 V 输入时 MOSFET 可用 V<sub>DS</sub> 余量约 0.60 V。下表给出 10 A 时的掉压估算与是否满足 1.8 V@10 A 的判据（满足：10 A 掉压 ≤ 0.60 V）：

- **IRFP4468PBF**（R10=2.6 mΩ）
  - Vth=2.0/3.0/4.0 V → R5≈6.9/9.1/15.6 mΩ → 10 A 掉压≈0.069/0.091/0.156 V → 满足（冗余大）。
- **IRFP4568PBF**（R10=5.9 mΩ）
  - Vth=3.0/4.0/5.0 V → R5≈20.7/35.4/∞ mΩ → 掉压≈0.207/0.354/∞ V → 典型满足；最差 Vth=5 V 下 5 V 栅驱可能无法可靠导通。
- **IRFP4668PBF**（R10=9.7 mΩ）
  - Vth=3.0/4.0/5.0 V → R5≈34.0/58.2/∞ mΩ → 掉压≈0.339/0.582/∞ V → 典型临界满足（≈0.582 V）；最差不满足。
- IRFP4227PBF（R10=25 mΩ）
  - Vth=3.0/4.0/5.0 V → R5≈87.5/150/∞ mΩ → 掉压≈0.875/1.50/∞ V → 1.8 V 端不满足 10 A。
- IRFP260NPBF（R10=40 mΩ）
  - Vth=2.0/3.0/4.0 V → R5≈106.7/140/240 mΩ → 掉压≈1.07/1.40/2.40 V → 1.8 V 端不满足 10 A。

据相同模型（取 Vth 典型值）估算在 1.8 V 工况下的“电压余量所容许的 5 V 栅驱最大电流”`
I_max ≈ 0.60 V / R5`：
- **IRFP4468PBF**（Vth≈3 V）：R5≈9.1 mΩ → I_max≈66 A（热/SOA 未计入，仅表征“电压余量”）。
- **IRFP4568PBF**（Vth≈4 V）：R5≈35.4 mΩ → I_max≈17.0 A。
- **IRFP4668PBF**（Vth≈4 V）：R5≈58.2 mΩ → I_max≈10.3 A（临界）。
- IRFP4227PBF（Vth≈4 V）：R5≈150 mΩ → I_max≈4.0 A。
- IRFP260NPBF（Vth≈3 V）：R5≈140 mΩ → I_max≈4.3 A。

> 说明：上述为一阶估算，未考虑迁移率退化、温升对 R<sub>DS(on)</sub> 的抬升、器件分散与米勒平台影响。特别地，IRFP4468 的门极平台电压（≈4.8 V）表明 5 V 驱动处于极小过驱区，需以台架测试（1.8–2.5 V 大电流）确认环路是否存在栅极饱和/纹波升高。

### 5 V 下的驱动电流与动态能力（OPA2365AIDR）

运放输出能力 ±65 mA（`docs/opamps/ti/OPA2365AIDR.md:183`）。按数据手册 Q<sub>g</sub>@10 V 近似估算 0–5 V 充电时间（保守取 `Q_0‑5V ≈ 0.5×Q_0‑10V`）：
- **IRFP4468PBF**：Q<sub>g</sub>(0–10 V)≈363 nC → 0–5 V 约 182 nC → t≈182 nC/65 mA≈2.8 µs。
- **IRFP4568PBF**：Q<sub>g</sub>≈151–227 nC（不同版本标注）→ 0–5 V 约 75–114 nC → t≈1.2–1.8 µs。
- **IRFP4668PBF**：Q<sub>g</sub>≈161 nC → 0–5 V ≈80 nC → t≈1.2 µs。
- **IRFP4227PBF**：Q<sub>g</sub>≈70–98 nC → 0–5 V ≈35–49 nC → t≈0.5–0.75 µs。
- **IRFP260NPBF**：Q<sub>g</sub>≈234 nC → 0–5 V ≈117 nC → t≈1.8 µs。

上述仅表征阶跃充放电时间常数量级（影响环路响应/纹波），闭环稳定性仍需配合栅串阻（10–20 Ω）与补偿网络实测整定（参考 `docs/opamps/ti/OPA2365AIDR.md:288-296`）。

### 对比表（5 V 栅驱，典型估算）

假设与口径：
- 以数据手册 `R_DS(on)@10 V` 与 `V_th(typ)` 估算 `R5`，采用 `R5 ≈ R10 × (10 − V_th) / (5 − V_th)`；忽略温升/分散/SOA。
- 低压端固定串联压降按当前设计假设：采样 0.25 V + 防反 0.8 V + 保险丝 0.15 V，共计 ≈1.20 V。
- “10 A 时最低负载电压” = 1.20 V + `R5×10 A`；“1.8 V 时最大负载电流” = `(1.8−1.20) V / R5`（仅电压约束）。

| 型号 | 10 A 时最低负载电压 | 1.8 V 时最大负载电流 |
| --- | ---: | ---: |
| **IRFP4468PBF** | ≈ 1.291 V | ≈ 66 A |
| **IRFP4568PBF** | ≈ 1.554 V | ≈ 17.0 A |
| **IRFP4668PBF** | ≈ 1.782 V | ≈ 10.3 A |
| IRFP4227PBF | ≈ 2.700 V | ≈ 4.0 A |
| IRFP260NPBF | ≈ 2.600 V | ≈ 4.3 A |

> 提示：IRFP4568/4668 的 V_th 上限可达 5 V（最差分档），此时 5 V 栅驱可能无法可靠完全导通；表中为典型估算，实际能力需在 1.8–2.5 V 台架实测确认。

## 选型结果与建议

1. **主 BOM**：**IRFP4468PBF**——兼顾价格、低掉压与易驱动，最适合 1.8 V~48 V 线性模式。  
2. **高压 / 冗余 SKU**：**IRFP4568PBF**（150 V）或 **IRFP4668PBF**（200 V），根据实际测试电压需求选用；在 5 V 栅驱下，**IRFP4668PBF** 低压大电流能力接近临界，建议优先验证 1.8–2.5 V 工况。  
3. **成本版本**：IRFP4227PBF（允许低压降额）或 IRFP260NPBF（显著降额运行）。部署前必须在 1.8 V 输入下测量最大可控电流，并在固件限功中设置专用曲线。  
4. **散热控制**：对 200 W 脉冲，软件需限制持续时间；必要时增强导风密封或换更高静压风扇，确保表面温度 <100 °C。  
5. **验证步骤**：  
   - 低压（1.8–2.5 V）大电流测试，确认 OPA2365 栅极不饱和、输出纹波可控。  
   - 散热器表面温度与功率积分联动测试，校准 100 W 连续、200 W 短时的限值。  
   - 根据不同 SKU 的掉压差异，调整固件功率限值与最低输入电压下的电流曲线。

> 注：若实际机械布局采用“TO‑247×2 并联 + TO‑220×1”方案，各项“单管”功率/电流能力需明确区分“每管指标/整机指标”，避免误解。

喵，这样就能在真实可购的列表里挑出兼顾线性性能、散热与性价比的 MOSFET 组合啦，后续只需按照建议完成实测和固件限功就可以把负载新版本稳稳上线。  
