# DDC 协议与功能

## 1. 文档范围

本文说明当前实现使用的 DDC/CI 地址、报文、响应、时序、错误语义和功能定义。代码中的 `crates/agon-osd/src` 是具体实现的直接依据。

## 2. 地址与正文

标准地址包含一个 VCP 字节。扩展地址包含两个前缀字节和一个子码，目前使用 `E2 A0` 与 `F8 E1` 两组前缀。

| 地址类型 | 读取正文 | 写入正文 | 读取长度 | 写入长度 |
|---|---|---|---:|---:|
| 标准 VCP | `01 VCP` | `03 VCP vH vL` | 2 | 4 |
| 扩展地址 | `01 P1 P2 sub` | `03 P1 P2 sub vH vL` | 4 | 6 |

`vH` 和 `vL` 分别是 16 位值的高字节和低字节。

## 3. 请求报文

完整请求由源地址、长度、正文和校验字节组成：

```text
51 (正文长度 | 80) <正文> checksum
```

校验从 `0x6E` 开始，依次异或 `0x51`、长度字节和每个正文字节，计算结果追加为报文的最后一个字节。

标准亮度读取示例：

```text
51 82 01 10 AC
```

扩展地址写入示例：

```text
51 86 03 E2 A0 20 vH vL checksum
```

## 4. 响应解析

当前实现按 11 字节 VCP 响应解析，并检查源地址、长度标志、负载长度、校验和及操作码。

| 下标 | 含义 |
|---:|---|
| 0 | 响应源地址，必须为 `6E` |
| 1 | 长度标志和负载长度，必须包含 `80`，负载长度为 8 |
| 2 | 操作码，必须为 `02` |
| 3 | 结果码，0 表示成功 |
| 4 | 标准地址的 VCP 回显 |
| 5 | 功能类型数据 |
| 6、7 | 最大值高字节、低字节 |
| 8、9 | 当前值高字节、低字节 |
| 10 | 校验字节 |

读取校验从 `0x50` 开始，异或全部 11 字节，结果必须为 0。标准地址检查响应中的 VCP 回显是否等于请求地址，扩展地址跳过该项检查。

非零结果码归类为 `Unsupported`。

### 4.1 能力串与设备值列表

能力串使用 `F3` 请求和 `E3` 响应按偏移分段读取。请求正文为 `F3 OFS(H) OFS(L)`，完整请求为：

```text
51 83 F3 OFS(H) OFS(L) checksum
```

响应包含源地址、长度、`E3`、两个字节的偏移回显、最多 32 字节数据和校验字节。读取端校验源地址、长度、操作码、偏移和校验和，再按实际数据长度更新下一段偏移。结束条件仅限 NUL 或空数据片段；短于 32 字节的片段仍需继续读取。

`DeviceCapabilities` 只解析 `vcp(...)` 中的地址和显式值列表。标准码使用两位十六进制文本；扩展码兼容 `E2A020(...)`、`F8E151(...)` 扁平形式以及命名空间括号形式。解析过程忽略 `max val` 字段。

`listed_values(address)` 直接返回设备上报的 `u8` 切片，完整保留原始顺序和重复项，返回内容独立于本地枚举和 `maximum`。`Feature::support_path()` 按功能码返回 `SupportPath::CapabilityList`、`SupportPath::Maximum` 或 `None`，分类规则集中由核心维护。`capabilities` 命令输出为 `current=N max=N support=capability-list|maximum|not-applicable capabilities=[...]`。空列表仅表示该地址缺少显式值列表，功能支持策略仍由 `Feature::support_path()` 决定。

## 5. 时序与重试

| 行为 | 当前规则 |
|---|---|
| 查询请求写入后、响应读取前 | 等待 50 毫秒 |
| 读取失败后的再次尝试 | 等待 20 毫秒 |
| 读取结束后的命令间隔 | 等待 50 毫秒 |
| 写入结束后 | 等待 100 毫秒 |
| 单次读取操作 | 最多尝试三次，所有通道共享该预算 |
| 固件明确拒绝地址 | 结束当前读取 |
| 通道缺少该地址能力 | 当前通道停止；多通道实现选择其他合格通道 |
| 写入传输失败 | 保持单次写入并返回错误 |
| 写入后的回读失败 | 保持单次写入并返回回读错误 |

多通道读取必须让一次请求及其响应在同一通道完成。恢复过程只更新通道选择，后续 I/O 由下一次读取执行。

## 6. 功能值与错误语义

功能值分为两类：

- `Number`：接受十进制或 `0x` 开头的十六进制 `u16` 数值，纯数值写入以显示器报告的最大值为上限。
- `Named`：接受功能表中的名称或完整 `u16` 数值。名称比较会去除首尾空白、转换为 ASCII 小写，并把下划线转换为连字符。静态功能共用固定表；HDR 和 LightFXMode 使用显示器回报的 `maximum` 选择名称表。

主要错误类型如下：

| 错误 | 含义 |
|---|---|
| `Transport` | 操作系统调用、短传输或其他传输失败 |
| `InvalidResponse` | 响应头、长度、操作码或地址回显校验失败 |
| `ChecksumMismatch` | 响应校验失败 |
| `Unsupported` | 显示器通过结果码拒绝地址 |
| `UnsupportedFeature` | 当前通道缺少该地址能力 |
| `UnsupportedCapabilities` | 当前通道拒绝能力串请求；与普通 VCP 地址错误独立 |
| `ValueAboveMaximum` | 纯数值输入超过显示器报告的最大值 |
| `WriteUncertain` | 写入传输异常，显示器可能已经应用该值 |
| `ReadbackFailed` | 写入调用完成，回读失败 |

遇到后两种写入状态时，先读取当前值，再决定后续操作。

## 7. 当前功能定义

下表列出代码公开的 55 项功能。`F8:E1:xx` 和 `E2:A0:xx` 分别表示两组扩展地址。数值功能仅接受数值输入；命名功能接受表内名称或 `u16` 数值。

| 功能名称 | 地址 | 类型或命名值 |
|---|---|---|
| `brightness` | `10` | 数值 |
| `contrast` | `12` | 数值 |
| `sharpness` | `87` | 数值 |
| `color-preset` | `14` | `none=0`、`srgb=1`、`warm=5`、`normal=6`、`cool=8`、`user-rgb=11`、`adobe-rgb=14` |
| `input-source` | `60` | `vga=1`、`dvi1=3`、`dp1=15`、`dp2=16`、`hdmi1=17`、`hdmi2=18`、`hdmi3=19`、`usb-c1=21`、`usb-c2=22`、`home=25` |
| `display-scaling` | `86` | `ratio-1-1=1`、`ratio-full-scale-max=2`、`ratio-full-scale-ratio=5`、`ratio-17-4-3=11`、`ratio-19-4-3=12`、`ratio-19-5-4=13`、`ratio-19w-16-10=14`、`ratio-215w-16-9=15`、`ratio-22w-16-10=16`、`ratio-23w-16-9=17`、`ratio-236-16-9=18`、`ratio-24w-16-9=19`、`ratio-27w-16-9=20`、`ratio-30w-21-9=21`、`ratio-32w-16-9=22`、`ratio-34w-21-9=23`、`ratio-40w-16-9=24`、`ratio-42w-16-9=25`、`ratio-movie1=33`、`ratio-movie2=34`、`ratio-aspect=35`、`ratio-full-16-9=39`、`ratio-1-1-16-9=40`、`ratio-full-square=41`、`ratio-1-1-square=42`、`ratio-24-5=43`、`ratio-27=44` |
| `volume` | `62` | 数值 |
| `mute` | `8D` | `on=1`、`off=2` |
| `game-mode` | `DC` | `off=0`、`fps=11`、`rts=12`、`racing=13`、`gamer1=14`、`gamer2=15`、`gamer3=16` |
| `gamma` | `F8:E1:12` | `gamma1=0`、`gamma2=1`、`gamma3=2`、`gamma4=3`、`gamma5=4`、`srgb=5` |
| `overdrive` | `F8:E1:51` | `off=0`、`weak=1`、`medium=2`、`strong=3`、`boost=4` |
| `hdr-mode` | `F8:E1:52` | 按 `maximum` 动态命名，见 §8.1 |
| `dcb-mode` | `F8:E1:08` | `off=0`、`full-enhance=33`、`nature-skin=34`、`sky-blue=35`、`green-field=36`、`auto-detect=37`、`demo=38` |
| `eco-mode` | `F8:E1:11` | `standard=0`、`text=1`、`internet=2`、`game=3`、`movie=4`、`sport=5`、`reading=6`、`uniformity=7` |
| `low-blue-mode` | `F8:E1:59` | `off=0`、`multimedia=1`、`internet=2`、`office=3`、`reading=4` |
| `low-input-lag` | `F8:E1:55` | 数值；上游取值名称仍待确认 |
| `shadow-control` | `F8:E1:58` | 数值 |
| `light-fx` | `F8:E1:60` | `off=0`、`low=1`、`medium=2`、`strong=3` |
| `light-fx-mode` | `F8:E1:61` | 按 `maximum` 动态命名，见 §8.2 |
| `dial-point` | `F8:E1:5A` | `off=0`、`on=1` |
| `frame-counter` | `F8:E1:5B` | `off=0`、`top-right=1`、`bottom-right=2`、`bottom-left=3`、`top-left=4` |
| `saturation-red` | `F8:E1:71` | 数值 |
| `saturation-yellow` | `F8:E1:72` | 数值 |
| `saturation-green` | `F8:E1:73` | 数值 |
| `saturation-cyan` | `F8:E1:74` | 数值 |
| `saturation-blue` | `F8:E1:75` | 数值 |
| `saturation-magenta` | `F8:E1:76` | 数值 |
| `hue-red` | `F8:E1:78` | 数值 |
| `hue-yellow` | `F8:E1:79` | 数值 |
| `hue-green` | `F8:E1:7A` | 数值 |
| `hue-cyan` | `F8:E1:7B` | 数值 |
| `hue-blue` | `F8:E1:7C` | 数值 |
| `hue-magenta` | `F8:E1:7D` | 数值 |
| `color-gamut` | `E2:A0:20` | `panel-native=0`、`ntsc=1`、`srgb=2`、`adobe-rgb=3`、`dci-p3=4`、`rec-2020=5`、`rec-709=6`、`dmode=7`、`rec-2020-hdr=8`、`dci-p3-hdr=9`、`off=10`、`on=11`、`adobe-rgb-d50=12`、`dci-p3-d50=13`、`display-p3=14`、`cmr-srgb=128`、`cmr-display-p3=129`、`cmr-display-dci-p3=130`、`cmr-display-dci-p3-d50=131`、`cmr-adobe-rgb=132`、`cmr-adobe-rgb-d50=133`、`cmr-rec-2020=134`、`cmr-rec-709=135`、`cmr-rec-2020-hdr=136`、`cmr-dci-p3-hdr=137`、`cmr-custom=138` |
| `local-dimming` | `E2:A0:18` | `off=0`、`on=1`、`weak=2`、`medium=3`、`strong=4` |
| `shadow-boost` | `E2:A0:45` | `off=0`、`level1=1`、`level2=2`、`level3=3`、`dynamic=4` |
| `sharp-shooter-size` | `E2:A0:06` | `1x=0`、`1.5x=1`、`2x=2`、`off=3` |
| `pixel-orbiting` | `E2:A0:34` | `off=0`、`weak=1`、`medium=2`、`strong=3`、`strongest=4` |
| `screen-saver` | `E2:A0:35` | `off=0`、`on=1`、`slow=2`、`fast=3` |
| `multi-logo-protection` | `E2:A0:4C` | `off=0`、`level1=1`、`level2=2`、`level3=3`、`level4=4` |
| `boundary-dimmer` | `E2:A0:4D` | `off=0`、`low=1`、`medium=2`、`strong=3` |
| `taskbar-dimmer` | `E2:A0:4E` | `off=0`、`low=1`、`medium=2`、`strong=3` |
| `adaptive-sync` | `E2:A0:40` | `off=0`、`on=1` |
| `auto-warning` | `E2:A0:43` | `off=0`、`on=1` |
| `lea` | `E2:A0:46` | `off=0`、`on=1` |
| `thermal-protection` | `E2:A0:4F` | `off=0`、`on=1` |
| `auto-pixel-refresh` | `E2:A0:61` | `off=0`、`on=1` |
| `race-assist` | `E2:A0:67` | `off=0`、`on=1` |
| `spotlight` | `E2:A0:68` | `off=0`、`low=1`、`medium=2`、`strong=3` |
| `night-sight` | `E2:A0:69` | `off=0`、`low=1`、`medium=2`、`strong=3` |
| `clear-sight` | `E2:A0:6A` | `off=0`、`on=1` |
| `life-glow` | `E2:A0:6B` | `off=0`、`on=1` |
| `auto-vision` | `E2:A0:6C` | `off=0`、`on=1` |
| `audio-output` | `E2:A0:7F` | `speaker=0`、`headphone=1` |
| `daisy-chain` | `E2:A0:21` | `off=0`、`clone=1`、`extend=2` |

功能可用性由显示器固件和当前通道决定，实际能力通过读取或 `capabilities` 探测确认。命令显示的 `capabilities` 列表直接来自设备能力串，与本节名称表相互独立。

## 8. 按 maximum 选择的命名值

HDR 和 LightFXMode 的多套名称表、选择条件及候选值统一由核心库维护。`features` 按条件列出全部变体；`get` 和 `capabilities` 使用本次读取的 `maximum` 显示当前值名称；`set` 使用写入前预读的同一个 `maximum` 解析名称。直接输入的 `u16` 数值绕过名称表，仍按命名功能的规则原样下发。

### 8.1 HDR

| 显示器回报的 `maximum` | 名称表 |
|---|---|
| `<= 1` | `off=0`、`auto=1` |
| `= 3` | `off=0`、`hdr-picture=1`、`hdr-movie=2`、`hdr-game=3` |
| 其他值 | `off=0`、`hdr-display=1`、`hdr-picture=2`、`hdr-movie=3`、`hdr-game=4`、`hdr-color=5`、`hdr-peak=6` |

### 8.2 LightFXMode

| `maximum` | 名称表 |
|---|---|
| `12` | `static=0`、`simple-shift=1`、`gradient-shift=2`、`simple-filling=3`、`1-way-filling=4`、`2-way-filling=5`、`breathing=6`、`motion-point=7`、`zoom=8`、`discolor=9`、`water-wave=10`、`flashing=11`、`demo=12` |
| `13` | `audio=0`、`static=1`、`simple-shift=2`、`gradient-shift=3`、`simple-filling=4`、`1-way-filling=5`、`2-way-filling=6`、`breathing=7`、`motion-point=8`、`zoom=9`、`discolor=10`、`water-wave=11`、`flashing=12`、`demo=13` |
| `14` | `audio1=0`、`audio2=1`、`static=2`、`dark-point-sweep=3`、`gradient-shift=4`、`spread-fill=5`、`drip-fill=6`、`spreading-drip-fill=7`、`breathing=8`、`light-point-sweep=9`、`zoom=10`、`rainbow=11`、`wave=12`、`flashing=13`、`demo=14` |
| `15` | `lol=0`、`audio1=1`、`audio2=2`、`static=3`、`dark-point-sweep=4`、`gradient-shift=5`、`spread-fill=6`、`drip-fill=7`、`spreading-drip-fill=8`、`breathing=9`、`light-point-sweep=10`、`zoom=11`、`rainbow=12`、`wave=13`、`flashing=14`、`demo=15` |
| `16` | `cs=0`、`audio1=1`、`audio2=2`、`static=3`、`dark-point-sweep=4`、`gradient-shift=5`、`spread-fill=6`、`drip-fill=7`、`spreading-drip-fill=8`、`breathing=9`、`light-point-sweep=10`、`zoom=11`、`rainbow=12`、`wave=13`、`flashing=14`、`demo=15` |

其他 `maximum` 仅接受直接数值输入，其 LightFXMode 名称表仍待确认。`maximum=16` 的变体来自 G-Menu 分派规则，其实机可达性仍待验证。
