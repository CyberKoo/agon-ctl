# macOS 后端

## 1. 文档范围

本文说明 macOS 后端的框架封装、显示器发现、服务匹配、I²C 传输、资源生命周期和验证方法。通用协议见《DDC 协议与功能》，模块关系见《架构与接口》。

实现面向 Apple Silicon 提供的 CoreDisplay、CoreGraphics 和 IOKit 接口。

## 2. 分层与模块职责

| 层级 | 模块 | 职责 |
|---|---|---|
| 原始绑定 | `agon-osd-macos-sys` | 框架链接、C 类型和函数声明 |
| 框架封装 | `framework/cf.rs` | CoreFoundation 所有权、字符串和字典访问 |
| 框架封装 | `framework/core_graphics.rs` | 在线外接显示器及厂商、型号、序列号查询 |
| 框架封装 | `framework/core_display.rs` | 显示器信息字典和 `IOAVService` I²C 调用 |
| 框架封装 | `framework/io_kit.rs` | IOKit 注册表对象、属性、父节点和迭代器 |
| 显示器实现 | `display.rs` | 显示器枚举、服务匹配和 `PacketTransport` 实现 |

原始绑定保持 ABI 定义。所有权规则和错误转换集中在框架封装。显示器业务规则集中在 `display.rs`。

## 3. 资源生命周期

`OwnedCf` 表示按 Create Rule 持有的 CoreFoundation 对象，在析构时调用 `CFRelease`。空指针构造返回空值。

`CfString` 和 `CfDictionary` 保存所有者及原始引用。字典读取前检查动态类型，字符串转换使用 UTF-8 缓冲区。

`IoObject` 表示一个已保留的 IOKit 对象，在析构时调用 `IOObjectRelease`。`IoIterator` 使用同一所有权规则管理迭代器。

`AvService` 持有 `IOAVServiceCreateWithService` 返回的对象，生命周期由 `OwnedCf` 管理。

FFI 会话在专用工作线程中创建、使用和销毁这些对象。

## 4. 显示器发现

### 4.1 在线外接显示器

`CGGetOnlineDisplayList` 先查询数量，再读取显示器 ID 列表，并通过 `CGDisplayIsBuiltin` 保留外接显示器。

每个显示器依次处理：

1. 通过 `CoreDisplay_DisplayCreateInfoDictionary` 取得信息字典。
2. 读取 `IODisplayLocation` 作为后续 IOKit 匹配键。
3. 通过 CoreGraphics 取得厂商 ID、型号 ID 和序列号。
4. 优先读取 `DisplayProductName` 的 `en_US` 名称，缺失时使用 `厂商:型号` 格式。
5. 使用显示器 ID 的十进制字符串作为 `DisplayInfo.id`。

缺少信息字典或 `IODisplayLocation` 时跳过该记录。

### 4.2 IOKit 服务匹配

`MacDisplay::connect` 按 IOService 平面的递归遍历顺序处理注册表。遇到 `AppleCLCD2` 或 `IOMobileFramebufferShim` 时记录其路径；后续服务沿用这个关联，直到遇到下一个帧缓冲节点。AV 服务可能位于帧缓冲节点的子树之外，因此实现遍历完整的 IOService 平面，再依据 `IODisplayLocation` 选择关联服务。

候选服务必须同时满足：

- 最近出现的帧缓冲节点路径与当前显示器的 `IODisplayLocation` 相同。
- 名称为 `DCPAVServiceProxy`。
- `Location` 属性为 `External`。
- 可以创建对应的 `IOAVService`。

实现仅在候选唯一时建立传输对象；零个或多个候选均跳过该显示器。

候选父节点的 `EPICProviderClass` 用于确定 I²C 从机地址：

| 父节点类型 | 从机地址 |
|---|---:|
| `AppleDCPMCDP29XX` | `0xB7` |
| 其他类型 | `0x37` |

## 5. I²C 传输

`MacDisplay` 保存显示器信息、`AvService` 和从机地址，并实现 `PacketTransport`。

写入操作首先检查报文长度和首字节 `0x51`，随后调用：

```text
IOAVServiceWriteI2C(service, slave, 0x51, packet[1..])
```

读取操作调用：

```text
IOAVServiceReadI2C(service, slave, 0, response)
```

`slave` 根据服务类型取 `0x37` 或 `0xB7`。写入的数据区地址为 `0x51`，读取偏移为 0。

返回状态为 0 表示成功，其余状态转换为 `TransportError::Io`，操作名称分别使用 `IOAVServiceWriteI2C` 和 `IOAVServiceReadI2C`。

标准地址和扩展地址的报文编解码、响应校验及重试由核心库完成。

## 6. 错误处理

`MacDisplayError` 区分两类错误：

| 错误 | 来源 |
|---|---|
| `CoreGraphics` | 在线显示器列表查询失败 |
| `Registry` | IOKit 注册表或迭代器访问失败 |

服务匹配失败时跳过对应显示器，其他显示器继续处理。对象在成功、失败和候选歧义路径上均由所有者类型负责释放。

## 7. 测试与验收

### 7.1 原生测试

在 macOS 上执行：

```bash
cargo test -p agon-osd-macos --locked
```

测试分为三类：

- 候选唯一性：零个候选、单个候选、多个候选及候选释放。
- CoreFoundation：空对象、字符串转换、动态类型和所有权释放。
- IOKit：目标子树遍历、路径前缀和迭代结束行为。

CoreFoundation 和 IOKit 测试仅依赖系统框架和测试工具链。

交叉编译检查使用：

```bash
cargo check --workspace --all-targets --locked --target aarch64-apple-darwin
```

### 7.2 硬件验收

- 单台外接显示器的标准 VCP、E2A0 和 F8E1 读取结果正确。
- 多台显示器可以按 ID 分别选中并恢复低风险设置。
- 服务匹配失败时终止该显示器的操作，其他显示器保持可用。
- `AppleDCPMCDP29XX` 与其他服务类型使用各自的从机地址。
- 成功、失败和候选歧义路径上的对象均能正确释放。
