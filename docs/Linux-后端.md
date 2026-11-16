# Linux 后端

## 1. 文档范围

本文说明 Linux 后端的 AOC 显示器探测、设备打开、原始包传输、错误处理和验证方法。协议见《DDC 协议与功能》，模块关系见《架构与接口》。

Linux 实现使用 `i2c-dev`，DRM sysfs 仅用于可选的命名增强。实现不增加 MCAPS 或其他显示器方言支持。

## 2. 模块职责

| 模块 | 职责 |
|---|---|
| `discovery.rs` | 总线枚举、探测编排和枚举诊断 |
| `display.rs` | `LinuxDisplay` 对象、显示器信息访问和 `PacketTransport` 委托 |
| `probe.rs` | 单次 AOC 协议探测及探测错误分类 |
| `edid.rs` | 独立通道上的 EDID 基本块读取与身份解析 |
| `naming.rs` | DRM 连接器筛选、总线关联和 `DisplayInfo` 生成 |
| `error.rs` | `LinuxDisplayError` 定义 |
| `i2c.rs` | 总线编号解析、设备文件、从机地址及原始包传输 |
| `agon-osd-linux-sys` | `I2C_SLAVE`、`I2C_SLAVE_FORCE` 常量和 `ioctl` 声明 |

`LinuxDisplay` 保存 `DisplayInfo` 和已打开的 `I2cDevice`。核心库为该类型自动提供 `DdcTransport` 实现。

## 3. 显示器枚举

### 3.1 AOC 总线探测

是否列出显示器完全由 `/dev/i2c-*` 上的协议探测决定，不依赖 DRM 连接器或厂商分支：

1. 枚举 `/dev` 中名称符合 `i2c-N` 的目录项，按总线编号排序并去重。
2. 以读写方式打开总线，通过 `I2C_SLAVE` 设置 DDC/CI 从机地址 `0x37`。
3. 使用核心 `DdcTransport` 发送一次标准亮度读取 `51 82 01 10 AC`，等待 50 ms 后读取 11 字节响应。
4. 响应通过现有 AOC 协议校验时保留该总线。显示器返回结构合法的非零结果码也证明协议通路存在，因此同样保留。
5. 无应答及其他普通传输错误静默跳过；完整读取后出现的响应结构或校验错误通过诊断回调报告。

探测直接调用单次 `DdcTransport::read_vcp`，不进入 `DdcClient` 的三次读取重试。正式功能调用仍沿用核心重试规则。

`/dev` 根目录不可读时返回 `LinuxDisplayError::Io`。单个总线失败不影响其他总线。

### 3.2 EDID 与命名增强

探测成功后，实现通过独立文件描述符在从机 `0x50` 上写入偏移 0 并读取 128 字节 EDID 基本块。该读取是 best-effort，不改变正式 DDC 通道的 `0x37` 地址，也不会因失败而丢弃已探测到的显示器。

随后读取 `/sys/class/drm/cardN-*`，仅将 `status == connected` 的外部连接器用于命名。`eDP`、`LVDS` 和 `DSI` 不参与命名。总线与连接器按以下顺序关联：

1. 若 `<connector>/ddc/i2c-dev/i2c-N` 存在，直接关联。
2. 否则比较 DDC 读取的 EDID 与连接器 EDID，厂商 ID、型号 ID 和序列号均相同且只有一个匹配项时关联。
3. 无匹配或匹配有歧义时保留总线 ID 名称，不影响探测结果。

各字段按以下规则生成：

| 字段 | 生成规则 |
|---|---|
| `id` | `i2c-N` |
| `name` | 关联成功后优先使用有效 EDID 型号名称，其次使用连接器名称；未关联时使用显示器 ID |
| `vendor_id`、`model_id`、`serial_number` | 来自关联连接器或该总线的有效 EDID；无可用 EDID 时使用默认值 |
| `location` | 对应的 `/dev/i2c-N` 路径 |

DRM 根目录、连接器状态或 EDID 不可读只会损失可选命名信息。sysfs 中 EDID 文件的元数据长度可能为 0，实现直接读取内容，不依赖元数据长度。

### 3.3 设备打开

设备通过 `OpenOptions` 以读写方式打开。打开成功后立即调用 `ioctl(fd, I2C_SLAVE, address)`；若返回 `EBUSY`，改用 `I2C_SLAVE_FORCE` 重试一次。

权限错误在所有总线间只汇总报告一次，提示将当前用户加入 `i2c` 组并重新登录。其他单总线打开失败按无可用显示器处理，继续枚举。

## 4. 原始包传输

`LinuxDisplay` 将 `PacketTransport` 委托给 `I2cDevice`：

- `write_packet` 检查报文至少包含两个字节，且首字节为 `0x51`，随后通过 `write_all` 写出完整报文。
- `read_packet` 通过 `read_exact` 读取调用方指定长度的响应。

传输保持 `I2C_SLAVE` 配合文件接口 `read()` / `write()`，不改用 `I2C_RDWR`。NVIDIA 专有驱动会拒绝部分 i2c-dev ioctl 组合，现有文件接口是必须保留的兼容路径。

文件描述符由 `File` 负责关闭。设备对象可以交由 FFI 会话的专用工作线程统一管理。

错误按以下方式转换：

| 系统结果 | `TransportError` |
|---|---|
| `io::ErrorKind::UnexpectedEof` | `UnexpectedEof` |
| 其他输入输出错误 | `Io`，包含操作名称和操作系统错误码 |

标准地址和扩展地址共用这一原始传输路径，地址编解码由核心库完成。

## 5. 诊断接口

`enumerate()` 返回显示器列表并丢弃可恢复的诊断信息。`enumerate_with_diagnostics(callback)` 通过回调报告局部失败。

诊断场景包括：

- `/dev` 或 DRM 中的单个目录项读取失败。
- DRM 根目录或连接器 `status` 不可读；这些情况只影响命名。
- `/dev/i2c-*` 权限不足，整个枚举过程只提示一次。
- 总线返回完整但不符合受支持 AOC 协议的响应。
- `/dev` 根目录不可读时返回 `LinuxDisplayError::Io`。

命令行程序负责打印这些诊断。FFI 使用静默枚举，错误通过状态码和错误对象返回。空列表统一表示未找到受支持的 AOC 外接显示器，不再把权限问题描述成设备不具备通用 DDC 能力。

## 6. 测试与验收

### 6.1 单元测试

执行：

```bash
cargo test -p agon-osd-linux --locked
```

测试分别放在 `discovery.rs`、`naming.rs` 和 `probe.rs` 中，共享 `test_support.rs` 提供的临时目录和 EDID 夹具。测试分别建立假的 `/dev/i2c-*` 列表和 DRM 连接器，探测结果通过闭包注入，诊断消息通过回调收集；命名夹具使用真实的 `ddc/i2c-dev/i2c-N` 目录结构，不再创建 `ddc` 符号链接。

覆盖项目包括：

- 合法与非法连接器名称，以及内置连接器排除。
- 没有 `<connector>/ddc` 节点时仍按 AOC 探测结果列出显示器。
- 单次标准亮度探测报文、合法响应及合法非零结果码。
- 直接总线关联、唯一 EDID 匹配、重复匹配和不合格连接器回退。
- 权限错误只提示一次，无应答总线保持静默。
- 非法 AOC 响应诊断及单个总线失败时保留其他结果。
- `/dev` 根目录读取失败返回错误，DRM 读取失败不丢弃显示器。

### 6.2 硬件验收

- 使用受支持的 AOC 外接显示器；其他品牌或协议不作为列出和功能验收目标。
- 无 `<connector>/ddc` 节点的 NVIDIA 环境仍能发现并通过 AOC 协议探测的显示器。
- 能关联 DRM 时名称和 EDID 身份正确；无法关联时仍列出 `i2c-N`。
- 权限不足时只出现一次明确提示，修正权限后设备可正常枚举。
- 标准 VCP、E2A0 和 F8E1 读取结果正确。
- 写入亮度等低风险设置后回读正确，恢复原值后再次确认。

跨目标编译检查使用：

```bash
cargo check --workspace --all-targets --locked --target aarch64-unknown-linux-gnu
```
