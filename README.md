# agon-ctl

`agon-ctl` 是一个用 Rust 编写的显示器 OSD 控制工具。它通过 DDC/CI 读取和修改显示器内置设置，支持标准 VCP，以及 AGON 使用的 `E2A0`、`F8E1` 扩展命令。

项目以命令行操作和脚本集成为主，适合日常调节、快速切换和批量管理显示器设置。

## 功能范围

当前提供 55 项 OSD 功能定义，完整名称可通过 `agon-ctl features` 查看。

| 类别 | 主要功能 |
|---|---|
| 基本显示设置 | 亮度、对比度、锐度、色温、显示比例、游戏模式 |
| 输入与音频 | 输入源、音量、静音、音频输出 |
| 图像与游戏 | Gamma、OverDrive、HDR、节能、低蓝光、低输入延迟、阴影控制 |
| 色彩 | 色域、六轴饱和度、六轴色相 |
| 灯效与辅助 | 灯效、灯效模式、准星、帧计数器、狙击镜 |
| 高级图像设置 | 局部调光、Shadow Boost、Adaptive Sync、SpotLight、NightSight、ClearSight、LifeGlow、AutoVision |
| 面板维护 | 像素偏移、屏保、多徽标保护、边界变暗、任务栏变暗、热保护、自动像素刷新 |
| 其他 | Daisy Chain 等 |

每项功能的开放情况由显示器固件和当前连接方式共同决定，程序通过运行时探测展示实际能力。

主要操作包括：

- 枚举外接显示器及其控制通道。
- 读取功能当前值和最大值。
- 修改功能值并回读结果。
- 检查常用命名值和数值范围。
- 在多台显示器中按 ID 或名称选择目标。
- 通过命令间隔和最多三次读取尝试提高通信稳定性；写入仅发送一次。

## 支持的平台

| 平台 | 实现方式 | 当前进展 |
|---|---|---|
| macOS（Apple Silicon） | CoreDisplay / IOKit | 已在 M4 和 AG326UZD2 上验证标准 VCP、E2A0、F8E1 读取，以及亮度修改和恢复 |
| Linux | DRM、`/dev/i2c-*` | 已通过 Linux x86_64 测试及 ARM64 编译检查；实机验证待完成 |
| Windows | dxva2、Intel IGCL、AMD ADL、NVIDIA NvAPI | 已在 Windows x64、AMD ADL 和 AG326UZD2 组合上验证 SDK 加载、显示器枚举、`capabilities` 探测以及读写操作；dxva2、Intel IGCL、NVIDIA NvAPI 待实机验证 |

macOS 后端面向 Apple Silicon，覆盖其原生显示控制接口。

Windows 后端在运行期按进程架构加载厂商 SDK，编译产物保持精简。

## 构建

项目使用 Rust 2024 edition，并要求 Rust 1.85 或更新版本。

在对应操作系统的源码目录执行：

```bash
cargo build --release --bin agon-ctl
```

产物位置：

- macOS / Linux：`target/release/agon-ctl`
- Windows：`target/release/agon-ctl.exe`

也可以明确指定目标：

```bash
# Windows x64
cargo build --release --target x86_64-pc-windows-msvc --bin agon-ctl

# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu --bin agon-ctl

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu --bin agon-ctl
```

交叉构建需要对应平台的 Rust target、链接器和运行库。`cargo check` 适合验证目标代码，`cargo build` 用于生成可执行文件。

## 运行要求

### macOS

- Apple Silicon Mac。
- 支持 DDC/CI 的外接显示器及兼容连接方式。
- 当前使用 CoreDisplay 中的 `IOAVService*` 接口，后续版本适配可集中由 macOS 后端维护。

### Linux

- 内核已加载 `i2c-dev`。
- 当前用户拥有目标 `/dev/i2c-*` 设备的读写权限。
- DRM 中显示为 `connected` 且存在 DDC 通道的连接器。
- 默认排除 eDP、LVDS、DSI 内置面板，单个连接器异常时告警并跳过。

### Windows

根据显卡驱动提供的用户态 SDK 运行：

| 后端 | x64 | x86 |
|---|---|---|
| Intel | `ControlLib.dll` | `ControlLib32.dll` |
| AMD | `atiadlxx.dll` | `atiadlxy.dll` |
| NVIDIA | `nvapi64.dll` | `nvapi.dll` |

程序从 System32 搜索路径加载这些库，并自动组合可用后端。可根据显卡型号预先安装对应驱动，程序启动后直接枚举和控制显示器。

标准 VCP 默认使用 dxva2，失败时可在读取的总计三次尝试内切换厂商原始通道；扩展命令仅使用原始通道。每次尝试都在同一通道完成请求与响应。单个输出失败时保留其他已发现结果，局部异常输出到 stderr。

Windows 按设备路径取得 dxva2 身份，并结合有效 EDID/序列号关联后端。多个同身份候选分别列出，通过 `--display` 选择目标。dxva2 ID 优先使用设备接口路径；`dxva2:fallback:process-…:token-…` 为当前进程的临时 ID。更换接口或重新安装驱动后，重新运行 `list` 获取 ID。

## 命令用法

### 查看显示器

```bash
agon-ctl list
```

输出包含显示器 ID、名称、厂商、型号、序列号和连接位置。

### 查看全部功能定义

```bash
agon-ctl features
```

该命令列出功能名称、DDC 地址以及可接受的命名值。HDR 和 LightFXMode 按 `maximum` 条件列出多套名称表；实际读取和写入时使用显示器本次回报的 `maximum` 选择对应表。

### 探测显示器功能

```bash
agon-ctl capabilities
```

程序先分段读取设备能力串，再逐项读取功能当前值和最大值，并汇总传输或响应异常。对于当前通道仅支持部分功能的情况（例如 dxva2 仅提供标准 VCP），程序跳过通道能力之外的项目并继续探测；这些跳过属于正常结果，命令退出状态由实际异常决定。

每行同时显示 `current`、`max`、`support` 和 `capabilities`：

```text
brightness: current=20 max=100 support=not-applicable capabilities=[]
color-gamut: current=0 (panel-native) max=4 support=capability-list capabilities=[0, 2, 4]
gamma: current=0 (gamma1) max=2 support=capability-list capabilities=[0, 1, 2, 3, 4]
hdr-mode: current=0 (off) max=3 support=maximum capabilities=[0, 1, 2, 3, 4, 6]
```

`support` 来自核心 `Feature::support_path()`：`SupportPath::CapabilityList` 输出 `capability-list`，表示按能力串显式列表判定；`SupportPath::Maximum` 输出 `maximum`，表示按 `maximum` 规则判定；其余功能输出 `not-applicable`。`capabilities` 是设备在该功能地址下显式上报的十进制值列表，完整保留设备原始顺序和内容，独立于本地名称表和 `maximum`。`capabilities=[]` 表示该地址缺少显式值列表；能力串读取失败或被当前通道拒绝时，命令另发警告。

### 读取功能

```bash
agon-ctl get brightness
agon-ctl get contrast
agon-ctl get color-gamut
agon-ctl get gamma
agon-ctl get overdrive
```

示例输出：

```text
brightness: current=24 max=100
color-gamut: current=0 (panel-native) max=4
gamma: current=0 (gamma1) max=2
```

### 修改功能

```bash
agon-ctl set brightness 40
agon-ctl set color-gamut dci-p3
agon-ctl set overdrive medium
agon-ctl set hdr-mode hdr-picture
agon-ctl set adaptive-sync on
agon-ctl set input-source dp1
agon-ctl set volume 20
```

数值可以使用十进制或 `0x` 开头的十六进制：

```bash
agon-ctl set brightness 0x28
```

亮度等纯数值功能以显示器返回的最大值为上限。命名型功能支持名称表中的名称和直接输入的 `u16` 数值，按输入值下发。

写入只发送一次，成功后再次读取并显示结果。若提示写入结果待确认，或写入完成但回读失败，设置可能已经生效。此时应先执行 `get` 检查，再根据当前值决定后续操作；输入源切换和面板维护操作尤其需要先确认状态。

### 多显示器选择

单台显示器可直接执行命令。多台显示器建议明确指定：

```bash
agon-ctl get brightness --display AG326UZD2
agon-ctl set brightness 40 --display 2
```

Linux 显示器 ID 通常为 `i2c-N`：

```bash
agon-ctl get brightness --display i2c-6
```

## 常用功能名称与取值

| 功能 | 示例 |
|---|---|
| 亮度 | `agon-ctl set brightness 40` |
| 对比度 | `agon-ctl set contrast 50` |
| 色域 | `agon-ctl set color-gamut panel-native` |
| 其他色域 | `srgb`、`adobe-rgb`、`dci-p3`、`display-p3`、`rec-709`、`rec-2020` |
| Gamma | `gamma1` 至 `gamma5`、`srgb` |
| OverDrive | `off`、`weak`、`medium`、`strong`、`boost` |
| 输入源 | `dp1`、`dp2`、`hdmi1`、`hdmi2`、`hdmi3`、`usb-c1`、`usb-c2` |
| 静音 | `agon-ctl set mute on` / `agon-ctl set mute off` |
| 游戏模式 | `off`、`fps`、`rts`、`racing`、`gamer1`、`gamer2`、`gamer3` |

Gamma 和 OverDrive 的显示名称采用官方 `UIType = 0` 的形式。其他型号可直接使用数值设置，命名差异与协议中的实际值保持独立。

## 使用建议

- 修改前使用 `get` 记录原值，便于恢复。
- 输入源切换、图像模式和面板维护操作前，先确认当前功能支持情况。
- 各类显示器的响应节奏存在差异，读取会自动有限重试；写入失败先检查当前状态，再决定是否手动重试。
- 能力探测以显示器返回为准，型号信息用于辅助识别和展示。
- 当前重点是 DDC/CI 控制、命令行操作和脚本集成。

## 项目结构

- `src/`：命令行入口、参数解析和平台选择。
- `crates/agon-osd`：OSD 功能定义、DDC 报文、校验、读取和写入逻辑。
- `crates/agon-osd-ffi`：C ABI、会话工作线程和快照对象。
- `crates/agon-osd-macos`：macOS 安全封装与传输。
- `crates/agon-osd-macos-sys`：macOS 原始 FFI 声明。
- `crates/agon-osd-linux`：Linux 设备枚举与传输。
- `crates/agon-osd-linux-sys`：Linux 原始 ioctl 声明。
- `crates/agon-osd-windows`：Windows SDK 动态加载、显示器合并与传输。
- `crates/agon-osd-windows-sys`：Intel、AMD、NVIDIA 的 ABI 类型与结构定义。

业务协议与平台 FFI 采用分层设计，原始绑定与 OSD 业务库保持独立。

## 开发验证

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo +1.85.0 check --workspace --all-targets --locked
```

CI 在 Linux、Windows、macOS 分别执行原生测试，并检查 Windows x86、macOS ARM64 和 Linux ARM64 目标。Windows SDK 模拟测试需要 Windows 系统 DLL，只在 Windows 运行；纯路由和身份策略测试可以跨平台运行。macOS 的 CF/IOKit 测试仅依赖原生系统框架和测试工具链。

架构、协议和测试说明分别见 [`docs/架构与接口.md`](docs/架构与接口.md)、[`docs/DDC-协议与功能.md`](docs/DDC-协议与功能.md) 和 [`docs/测试与验收.md`](docs/测试与验收.md)。平台实现说明见 [`docs/Windows-后端.md`](docs/Windows-后端.md)、[`docs/Linux-后端.md`](docs/Linux-后端.md) 和 [`docs/macOS-后端.md`](docs/macOS-后端.md)。

## 授权协议

本项目采用 [BSD 3-Clause License](LICENSE) 授权协议。
