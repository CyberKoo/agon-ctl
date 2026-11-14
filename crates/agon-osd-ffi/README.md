# agon-osd-ffi

AGON 显示器控制的 C ABI，支持 macOS、Windows 和 Linux。

| 编译平台 | 控制后端 |
|---|---|
| macOS | `agon-osd-macos`：CoreDisplay / IOKit |
| Windows | `agon-osd-windows`：dxva2 / IGCL / ADL / NvAPI |
| Linux | `agon-osd-linux`：DRM / i2c-dev |

## 构建

在仓库根目录执行：

```sh
cargo build -p agon-osd-ffi --profile ffi --locked
```

头文件：`include/agon_osd.h`。

静态库：macOS/Linux 为 `target/ffi/libagon_osd_ffi.a`，Windows MSVC 为 `target/ffi/agon_osd_ffi.lib`。指定 `--target` 后产物位于 `target/<target>/ffi/`。

`ffi` profile 启用 unwind，C API 将 Rust panic 转换为错误码。最终链接所需的系统库可通过以下命令查询：

```sh
cargo rustc -p agon-osd-ffi --profile ffi --locked -- --print native-static-libs
```

## 调用流程

1. `agon_session_create`：创建空会话和专用工作线程。
2. `agon_session_refresh`：通过当前平台后端枚举显示器，返回列表快照。
3. `agon_display_list_get`：获取显示器 ID 与信息。
4. `agon_read`、`agon_set`、`agon_probe`、`agon_feature_capabilities`：按显示器 ID 和功能名称操作。
5. `agon_display_list_destroy`、`agon_capability_list_destroy`、`agon_error_destroy`：释放快照、能力列表与错误对象。
6. `agon_session_destroy`：结束工作线程并释放设备资源。

三平台共用以上接口。设备对象均在工作线程中创建、使用和销毁。创建会话仅初始化工作线程，刷新负责枚举；刷新失败保留上一次成功枚举的设备。旧快照在后续刷新和会话销毁后仍可读取。

平台枚举的可恢复警告由调用方选择处理方式；当前 FFI 采用静默枚举，致命错误仍通过状态码和错误对象返回。

`agon_feature_count` 和 `agon_feature_get` 提供全部功能定义，可独立于会话调用。

`agon_feature_support_path` 返回功能的支持值判定策略，仅查询核心元数据：`AGON_SUPPORT_PATH_CAPABILITY_LIST` 对应能力串显式列表，`AGON_SUPPORT_PATH_MAXIMUM` 对应 `maximum` 规则，`AGON_SUPPORT_PATH_NOT_APPLICABLE` 表示不适用该分类。

所有功能的命名候选表统一通过以下流程获取：

1. 使用 `agon_read` 或 `agon_probe` 取得 `agon_value_t.maximum`。
2. 调用 `agon_feature_named_values(feature_index, maximum, ...)`，取得独立拥有的 `agon_named_value_list_t`。
3. 使用 `agon_named_value_list_count` 和 `agon_named_value_list_get` 读取候选项；名称在列表销毁前有效。
4. 使用 `agon_named_value_list_destroy` 释放列表。静态功能可直接使用任意 `maximum`，纯数值功能返回空列表。

命名值接口只负责元数据选择，可独立于会话调用。前端根据候选项展示名称，写回时继续调用 `agon_set` 并传入对应的 `uint16_t` 数值。

设备显式能力列表通过 `agon_feature_capabilities` 获取，再使用 `agon_capability_list_count/get` 读取 `uint8_t` 数值，最后调用 `agon_capability_list_destroy` 释放。列表完整保留设备原始顺序和内容，独立于本地名称表和 `maximum`；空列表表示该地址缺少显式值列表。返回列表独立拥有数据，在后续设备调用和会话销毁后仍可使用。

写入发送一次并回读结果。`AGON_WRITE_UNCERTAIN` 表示写入结果待确认；`AGON_READBACK_FAILED` 表示写入完成后回读失败。

字符串和对象的所有权、线程规则与完整函数签名见头文件。

## 测试

```sh
cargo test -p agon-osd-ffi --locked
cargo clippy -p agon-osd-ffi --all-targets --locked -- -D warnings
```

`examples/smoke.c` 提供 C 链接测试，覆盖元数据、空会话生命周期和错误对象释放。
