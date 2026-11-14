# Linux 后端

## 1. 文档范围

本文说明 Linux 后端的显示器枚举、设备打开、原始包传输、错误处理和验证方法。通用协议见《DDC 协议与功能》，模块关系见《架构与接口》。

Linux 实现使用 DRM sysfs 和 `i2c-dev`。实现范围限定为该访问路径。

## 2. 模块职责

| 模块 | 职责 |
|---|---|
| `display.rs` | 连接器筛选、EDID 信息整理、显示器枚举和 `PacketTransport` 实现 |
| `i2c.rs` | 设备文件打开、从机地址设置及完整报文读写 |
| `agon-osd-linux-sys` | `I2C_SLAVE` 常量和 `ioctl` 声明 |

`LinuxDisplay` 保存 `DisplayInfo` 和已打开的 `I2cDevice`。核心库为该类型自动提供 `DdcTransport` 实现。

## 3. 显示器枚举

### 3.1 连接器筛选

枚举从 `/sys/class/drm` 开始，依次处理目录项：

1. 按 `cardN-TYPE-N` 形式解析连接器名称，支持复合接口类型和 DP MST 后缀。
2. 跳过 `eDP`、`LVDS` 和 `DSI` 内置连接器。
3. 读取 `status`，仅处理内容为 `connected` 的连接器。
4. 解析连接器的 `ddc` 符号链接，从目标名称取得 `i2c-N` 总线编号。
5. 按总线编号去重，并生成 `/dev/i2c-N` 设备路径。

输出按总线编号排序。相同总线上的后续记录直接跳过。

### 3.2 显示器信息

实现读取连接器目录中的 `edid`，通过核心 EDID 解析取得名称、厂商 ID、型号 ID 和序列号。

各字段按以下规则生成：

| 字段 | 生成规则 |
|---|---|
| `id` | `i2c-N` |
| `name` | 优先使用 EDID 型号名称，其次使用连接器名称，最后使用显示器 ID |
| `vendor_id`、`model_id`、`serial_number` | 来自有效 EDID；解析失败时使用默认值 |
| `location` | 对应的 `/dev/i2c-N` 路径 |

EDID 缺失或解析失败时，枚举继续使用默认身份信息。

### 3.3 设备打开

设备通过 `OpenOptions` 以读写方式打开。打开成功后立即调用 `ioctl(fd, I2C_SLAVE, 0x37)`，将 DDC/CI 从机地址设为 `0x37`。

设备打开失败时通过诊断回调报告，并继续处理其他连接器。根目录读取失败时返回 `LinuxDisplayError::Io`。

## 4. 原始包传输

`LinuxDisplay` 实现 `PacketTransport` 的两个操作：

- `write_packet` 检查报文至少包含两个字节，且首字节为 `0x51`，随后通过 `write_all` 写出完整报文。
- `read_packet` 通过 `read_exact` 读取调用方指定长度的响应。

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

- 单个目录项读取失败。
- `status` 或 `ddc` 读取、解析失败。
- 设备打开失败。

命令行程序负责打印这些诊断。FFI 使用静默枚举，错误通过状态码和错误对象返回。

## 6. 测试与验收

### 6.1 单元测试

执行：

```bash
cargo test -p agon-osd-linux --locked
```

测试在临时目录中建立连接器、`status` 文件和 `ddc` 符号链接，通过闭包替换设备打开，通过回调收集诊断。

覆盖项目包括：

- 合法与非法连接器名称。
- 内置连接器排除和总线编号去重。
- 断开、文件缺失、无效链接和权限错误。
- 单个设备失败时保留其他结果。
- 根目录读取失败和诊断内容。

### 6.2 硬件验收

- 内置面板与外部显示器同时存在时，仅列出外部显示器。
- 每个 `/dev/i2c-N` 对应预期的物理连接器。
- 权限不足或设备拔插时，其他设备继续枚举。
- 标准 VCP、E2A0 和 F8E1 读取结果正确。
- 写入亮度等低风险设置后回读正确，恢复原值后再次确认。

跨目标编译检查使用：

```bash
cargo check --workspace --all-targets --locked --target aarch64-unknown-linux-gnu
```
