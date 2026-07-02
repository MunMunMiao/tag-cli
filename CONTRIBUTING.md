# Contributing to tag-cli

感谢你对 tag-cli 的兴趣。本指南涵盖项目架构、开发环境搭建、测试、子模块更新、代码风格、本地安装以及提交信息规范。

## Project architecture

本仓库是一个 Cargo workspace，由三个 crate 组成：

| Crate | 路径 | 职责 |
|-------|------|------|
| `tag-cli` | `crates/tag-cli` | 命令行入口、参数解析、命令分发、补全与 man 页生成 |
| `tag-core` | `crates/tag-core` | 共享逻辑：manifest 解析、工作流编排、封面处理、输出格式化、错误类型 |
| `taglib-rs` | `crates/taglib-rs` | TagLib C++ FFI 包装：读取/写入标签与封面、音频属性、测试工具 |

依赖关系：

```text
       tag-cli
       /    \
  tag-core  taglib-rs
      |
  taglib-rs
```

`tag-cli` 通过 `tag-core` 构建工作流（读取元数据 → 检测格式 → 处理封面 → 更新标签 → 解析输出路径 → 保存文件），最终调用 `taglib-rs` 完成与 TagLib 的交互。

> 注：`crates/tag-cli/build.rs` 与 `crates/tag-core/build.rs` 都包含链接参数，这是为了兼容 macOS 等平台的静态链接需求而做的重复；修改链接选项时需要同步检查两者。

## Development setup

开发前请准备以下依赖：

- Rust stable toolchain（MSRV 1.85+）
- C++ 工具链
- `cmake`
- `zlib` 开发头文件
- `ffmpeg`（仅集成测试需要，用于生成测试样本）

克隆仓库并构建：

```bash
git clone https://github.com/MunMunMiao/tag-cli.git
cd tag-cli
git submodule update --init --recursive
cargo build --release
```

首次完整构建会编译 vendored TagLib C++ 库，耗时约 3–10 分钟（取决于 CPU），`target/` 目录可能占用数 GB 磁盘空间。

### 平台依赖安装示例

- **Debian / Ubuntu**：
  ```bash
  sudo apt-get update && sudo apt-get install -y cmake zlib1g-dev
  # 源码构建还需要 clang 或 build-essential
  sudo apt-get install -y clang ffmpeg
  ```

- **macOS**：
  ```bash
  brew install cmake ffmpeg
  # 若提示缺少 C++ 头文件，请运行：
  xcode-select --install
  ```

## Running tests

```bash
# 单元测试与集成测试（需要 ffmpeg 在 PATH 中）
cargo test --workspace

# 仅运行单元测试（不依赖 ffmpeg）
cargo test --workspace --lib

# 运行特定集成测试
cargo test -p tag-cli --test cli_test
cargo test -p taglib-rs --test wrapper
```

集成测试会生成临时音频/图片样本，默认可并行运行；若遇到资源竞争，可尝试 `cargo test --workspace -- --test-threads=1`。

### 运行 `cargo test` 时提示找不到 ffmpeg

集成测试使用 ffmpeg 生成音频与图片样本。请安装 ffmpeg：

```bash
# Debian/Ubuntu
sudo apt-get install -y ffmpeg

# macOS
brew install ffmpeg
```

## Updating vendored TagLib

`vendor/taglib` 是 Git 子模块。更新上游版本需要：

```bash
cd vendor/taglib
git fetch origin
git checkout <new-tag>
cd ../..
git add vendor/taglib
```

更新后请完整运行 `cargo test --workspace`，因为 TagLib 的 ABI 或行为变化可能需要同步调整 FFI 绑定。

## Code style

提交前请检查代码格式与 Clippy 警告：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Local install

```bash
cargo install --path crates/tag-cli
```

安装后的二进制文件位于 `~/.cargo/bin/tag-cli`，请确保该目录在 `PATH` 中。

## Commit message convention

提交信息建议使用 `type(scope): description` 格式，例如：

- `feat(cli): add --replace flag`
- `docs(readme): update installation notes`
- `fix(taglib-rs): handle null cover data`
- `test(apply): add dry-run failure cases`

常用 type：

- `feat`：新功能
- `fix`：修复问题
- `docs`：文档变更
- `test`：测试相关
- `chore`：构建、工具、依赖等杂项
