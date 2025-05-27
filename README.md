# PortForward - 高性能 Rust 端口转发工具
PortForward - High-performance Rust Port Forwarding Tool

Rust Tokio

## 功能特性 | Features
* 🚀 纯 Rust 编写，基于 Tokio 的异步高性能实现
* Written in pure Rust with Tokio async runtime

* 💻 命令行界面操作简单
*  Simple command-line interface

* 🛠️ 支持安装为 Windows 服务（后台运行）
* Supports installation as Windows Service

* 📁 自动识别应用目录的配置和日志文件
* Auto-detects config/log files in executable directory

* ⚙️ 简洁的 TOML 格式配置文件
* Clean TOML configuration format

* 🔒 当前仅支持 TCP 协议转发
* TCP protocol only (currently)

## 安装与使用 | Installation & Usage

### 基本命令 | Basic Commands

PortForward [OPTIONS] <COMMAND>
选项参数 | Options:

-d, --daemon <DAEMON>: 运行模式 [app|service] (默认: app)
Operation mode [app|service] (default: app)

-c, --config <CONFIG>: 配置文件路径 (默认: config.toml)
Config file path (default: config.toml)

-l, --log <LOG>: 日志文件路径 (默认: PortForward.log)
Log file path (default: PortForward.log)

### 子命令 | Subcommands:

* install: 安装为 Windows 服务

Install as Windows Service

* uninstall: 卸载 Windows 服务

Uninstall Windows Service

* help: 显示帮助信息
Show help message

## 配置文件示例 | Config Example (config.toml)

<code>
[[forwards]]
name = "VNC转发"        # 转发规则名称
local_addr = "172.18.1.3:25001"  # 本地监听地址
remote_addr = "172.18.1.1:5901"  # 目标远程地址

[[forwards]]
name = "Web服务转发"    # Forwarding rule name
local_addr = "172.18.1.3:25002"  # Local listen address
remote_addr = "172.18.1.6:9000"  # Target remote address
</code>

## 使用说明 | Instructions

直接运行模式 | Direct Run:

<code>
PortForward -c /path/to/config.toml
</code>

安装服务 | Install Service:
<code>
PortForward install
</code>

卸载服务 | Uninstall Service:
<code>
PortForward uninstall
</code>
## 注意事项 | Notes
* 配置文件默认位置：程序所在目录的 config.toml
Default config path: config.toml in executable directory

* 日志会记录到程序目录的 PortForward.log
Logs are written to PortForward.log

* 当前版本仅支持 TCP 协议
TCP protocol only in current version
