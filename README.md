# 🦀 Blues Zencore

<div align="center">

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-AGPL--3.0-blue?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey?style=for-the-badge)

**Minimalist and Interactive Music Backup Tool**

Fast • Secure • Cross-Platform • Beautiful CLI

[Features](#-features) •
[Installation](#-installation) •
[Quick Start](#-quick-start) •
[Documentation](#-documentation) •
[Contributing](#-contributing)

</div>

---

## 🎯 What is Blues Zencore?

Blues Zencore adalah tool backup musik dan file berbasis Rust yang dirancang untuk:

- 🚀 **Blazingly Fast** - Parallel processing, SIMD optimization, ~360 MB/s compression
- 🔒 **Military-Grade Security** - AES-256-GCM encryption, Argon2 key derivation
- 🎨 **Beautiful UX** - Interactive CLI dengan fuzzy finding dan progress bars
- 🌍 **Cross-Platform** - Native support untuk Linux, macOS, dan Windows
- 📦 **Smart Archive Management** - Auto-naming, duplicate detection, state tracking
- 🔍 **Content Inspection** - Lihat isi archive tanpa extract

### Why Zencore?

### Using Rust?
-Rust is faster than Python
```bash
# Python version (old):  ~30 seconds, 220 MB/s
# Rust version (new):    ~17 seconds, 360 MB/s ⚡ +64% faster!
```

---

## ✨ Features

### Core Features

- ✅ **Multiple Compression Formats**
  - tar.zst (Zstandard - Recommended)
  - tar.gz (gzip)
  - zip (Universal)
  - Configurable compression levels

- ✅ **Advanced Encryption**
  - AES-256-GCM (Hardware accelerated)
  - ChaCha20-Poly1305 (Software, constant-time)
  - Argon2 password hashing
  - Auto-detect cipher from file

- ✅ **Multiple Hash Algorithms**
  - BLAKE3 (~7 GB/s, parallel)
  - SHA-256 (~500 MB/s, standard)
  - SHA3-256 (~200 MB/s, NIST)

- ✅ **Interactive Experience**
  - Fuzzy folder selection
  - Algorithm picker with descriptions
  - Real-time progress bars
  - Colored output

- ✅ **Smart Features**
  - Auto-generate names with timestamps
  - Duplicate detection (adds .1, .2, etc.)
  - State tracking (see archive contents without extract)
  - SHA-256/BLAKE3 automatic verification

- ✅ **Cross-Platform**
  - Linux (Ubuntu, Arch, Fedora, etc.)
  - macOS (Intel & Apple Silicon)
  - Windows 10/11 (Native support)
  - OS-specific path handling

### Performance

| Feature | Performance |
|---------|-------------|
| **Compression** | 150-500 MB/s (depends on algorithm) |
| **Checksums** | 7 GB/s (BLAKE3, parallel) |
| **File Scanning** | Parallel (uses all CPU cores) |
| **Memory Usage** | ~500 MB (efficient streaming) |

---

## 📦 Installation

### Prerequisites

- **Rust 1.70+** (for building from source)
- **System dependencies:**
  - Linux: `build-essential`, `pkg-config`, `libssl-dev`
  - macOS: Xcode Command Line Tools
  - Windows: MSVC Build Tools

### Quick Install

#### Linux / macOS

```bash
# Clone repository
git clone https://github.com/Blues24/Zencore-rs.git
cd Zencore-rs

# Build release binary
cargo build --release

# Install to /usr/local/bin (optional)
sudo cp target/release/zencore /usr/local/bin/

# Or add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

#### Windows

```powershell
# Clone repository
git clone https://github.com/Blues24/Zencore-rs.git
cd Blues-Zencore

# Build release binary
cargo build --release

# Binary located at: target\release\zencore.exe
# Add to PATH or run directly
```

### Pre-compiled Binaries

Download from [GitHub Releases](https://github.com/Blues24/Zencore-rs/releases):

- `zencore-linux-x86_64.tar.gz`
- `zencore-macos-universal.tar.gz`
- `zencore-windows-x86_64.zip`

---

## 🚀 Quick Start

### Interactive Mode (Easiest)

```bash
# Just run it!
zencore

# Output:
# 🎵 What would you like to do?
# > Create Backup
#   List Archives
#   Show Archive Contents
#   Exit
```

### Command Line

```bash
# Quick backup (will prompt for destination & algorithm)
zencore backup

# Full control
zencore backup \
  -s ~/Music \
  -d ~/Backups \
  -n "my_music_2024" \
  -a tar.zst \
  -e  # encrypt

# List all archives
zencore list

# Show archive contents (without extracting!)
zencore show my_music_2024.tar.zst

# Verify integrity
zencore verify ~/Backups/my_music_2024.tar.zst
```

### Configuration

First run creates config at:
- **Linux:** `~/.config/zencore/config.toml`
- **macOS:** `~/Library/Application Support/zencore/config.toml`
- **Windows:** `%APPDATA%\zencore\config.toml`

**Example config:**

```toml
# Fast compression with auto-threading
default_algorithm = "tar.zst"
compression_level = 3
num_threads = 0  # Auto-detect CPU cores

# Security
encrypt_by_default = false
default_cipher = "aes256"
default_hash_algorithm = "blake3"

# Default backup location
default_backup_destination = "~/Backups/Music"
```

---

## 📖 Documentation

Comprehensive documentation available in the [`docs/`](docs/) folder:

### Getting Started
- 📘 [**Installation Guide**](docs/INSTALLATION.md) - Detailed setup instructions
- 🚀 [**Quick Start Guide**](docs/QUICKSTART.md) - Get up and running in 5 minutes
- ⚙️ [**Configuration Guide**](docs/CONFIGURATION.md) - Complete config options

### User Guides
- 📝 [**User Manual**](docs/USER_MANUAL.md) - Complete feature documentation
- 💡 [**Use Cases**](docs/USE_CASES.md) - Real-world scenarios
- ❓ [**FAQ**](docs/FAQ.md) - Frequently asked questions

### Platform-Specific
- 🐧 [**Linux Guide**](docs/LINUX.md) - Linux-specific tips
- 🍎 [**macOS Guide**](docs/MACOS.md) - macOS-specific tips
- 🪟 [**Windows Guide**](docs/WINDOWS.md) - Windows support & tips

### Advanced
- ⚡ [**Performance Guide**](docs/PERFORMANCE.md) - Optimization & benchmarks
- 🔒 [**Security Guide**](docs/SECURITY.md) - Encryption & best practices
- 🛠️ [**Troubleshooting**](docs/TROUBLESHOOTING.md) - Common issues & solutions

### Developer
- 🏗️ [**Architecture**](docs/ARCHITECTURE.md) - Code structure & design
- 🤝 [**Contributing**](docs/CONTRIBUTING.md) - How to contribute
- 📋 [**API Reference**](docs/API.md) - Internal API documentation

### Reference
- 📚 [**Command Reference**](docs/COMMAND_REFERENCE.md) - All CLI commands
- 🔄 [**Backup Flow**](docs/BACKUP_FLOW.md) - Visual flow diagrams
- 📊 [**Comparison**](docs/COMPARISON.md) - vs other backup tools

---

## 🎯 Use Cases

### Daily Music Backup

```bash
# One-time setup
echo 'default_backup_destination = "~/Backups/Music"' >> ~/.config/zencore/config.toml

# Then just:
zencore backup
# Hit Enter 3 times → Done in 20 seconds!
```

### Scheduled Backups

**Linux/macOS (cron):**
```bash
# Weekly backup every Sunday at 2 AM
0 2 * * 0 /usr/local/bin/zencore backup -s ~/Music -d ~/Backups -a tar.zst
```

**Windows (Task Scheduler):**
```powershell
$action = New-ScheduledTaskAction -Execute "zencore.exe" `
  -Argument "backup -s %USERPROFILE%\Music -d D:\Backups"
$trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek Sunday -At 2am
Register-ScheduledTask -Action $action -Trigger $trigger -TaskName "Music Backup"
```

### Encrypted Archival

```bash
# Maximum compression + encryption
zencore backup \
  -s ~/Music \
  -d ~/Backups \
  -n "archive_$(date +%Y)" \
  -a tar.zst \
  -e
# Edit config: compression_level = 19
```

---

## 🔧 Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench

# Format code
cargo fmt

# Lint
cargo clippy
```

### Project Structure

```
blues-zencore/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli.rs               # CLI interface
│   ├── compress.rs          # Compression logic
│   ├── crypto.rs            # Basic encryption
│   ├── crypto_extended.rs   # Advanced encryption
│   ├── config.rs            # Configuration
│   ├── fuzzer.rs            # Fuzzy finder
│   ├── state.rs             # State tracking
│   ├── archive_name.rs      # Smart naming
│   ├── path_utils.rs        # Cross-platform paths
│   └── utils.rs             # Utilities
├── docs/                    # Documentation
├── benches/                 # Benchmarks
├── tests/                   # Integration tests
├── Cargo.toml               # Dependencies
└── README.md                # This file
```

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

### Ways to Contribute

- 🐛 [Report bugs](https://github.com/Blues24/Blues-Zencore/issues/new?template=bug_report.md)
- 💡 [Suggest features](https://github.com/Blues24/Blues-Zencore/issues/new?template=feature_request.md)
- 📖 Improve documentation
- 🔧 Submit pull requests
- ⭐ Star the repository

### Development Workflow

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

---

## 📊 Comparison

| Feature | Zencore | 7-Zip | tar+gzip | WinRAR |
|---------|---------|-------|----------|--------|
| Speed | ⚡⚡⚡ | ⚡⚡ | ⚡ | ⚡⚡ |
| Cross-platform | ✅ | ✅ | ✅ | ❌ |
| Interactive CLI | ✅ | ❌ | ❌ | ❌ |
| Encryption | ✅ AES-256 | ✅ AES-256 | ❌ | ✅ AES-256 |
| State tracking | ✅ | ❌ | ❌ | ❌ |
| Auto-verify | ✅ | ❌ | ❌ | ✅ |
| Open source | ✅ AGPL-3.0 | ✅ LGPL | ✅ GPL | ❌ |
| Free | ✅ | ✅ | ✅ | ❌ Trial |

---

## 🎓 Learning Resources

- 📺 [Video Tutorial](docs/TUTORIAL.md) (Coming soon)
- 📝 [Blog Post: Why Rust for Backup Tools?](docs/BLOG.md)
- 🎤 [Talk: Building Fast CLI Tools with Rust](docs/TALKS.md)

---

## 🐛 Troubleshooting

Common issues and solutions:

### "Config file not found"
```bash
# Auto-generates on first run
zencore

# Or manually check location
zencore config
```

### "Permission denied"
```bash
# Linux/macOS: Check permissions
chmod +x zencore
sudo chown $USER:$USER ~/.config/zencore

# Windows: Run as Administrator
```

### Slow performance
```toml
# Optimize config:
num_threads = 0
compression_level = 3
default_algorithm = "tar.zst"
```

More solutions: [Troubleshooting Guide](docs/TROUBLESHOOTING.md)

---

## 📜 License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

See [LICENSE](LICENSE) for details.

**What this means:**
- ✅ Free to use, modify, and distribute
- ✅ Must share source code if modified
- ✅ Must use same license for derivatives
- ✅ Network use = distribution (must share source)

---

## 🙏 Acknowledgments

Built with amazing open-source projects:
- [Rust](https://www.rust-lang.org/) - Safe systems programming language
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [dialoguer](https://github.com/console-rs/dialoguer) - Interactive prompts
- [indicatif](https://github.com/console-rs/indicatif) - Progress bars
- [rayon](https://github.com/rayon-rs/rayon) - Parallel processing
- [zstd](https://github.com/facebook/zstd) - Fast compression
- [ring](https://github.com/briansmith/ring) - Cryptography

---

## 📞 Contact & Support

- 🐛 **Bug Reports:** [GitHub Issues](https://github.com/Blues24/Zencore-rs/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/Blues24/Zencore-rs/discussions)
- 📧 **Email:** lukmanaffandi900@gmail.com


---

## 🗺️ Roadmap

### v1.0.0 (Current)
- ✅ Core backup functionality
- ✅ Multiple compression formats
- ✅ Advanced encryption
- ✅ Cross-platform support
- ✅ State tracking

### v1.1.0 (Planned)
- [ ] Restore functionality
- [ ] Incremental backups
- [ ] Cloud integration (rclone)
- [ ] Archive comparison
- [ ] GUI version (Tauri)

### v2.0.0 (Future)
- [ ] Differential backups
- [ ] Deduplication
- [ ] Archive splitting
- [ ] Remote backups (SSH/SFTP)
- [ ] Plugin system

See [ROADMAP.md](docs/ROADMAP.md) for detailed plans.

---

## ⭐ Show Your Support

If you find this project useful, please consider:
- ⭐ Starring the repository
- 🐦 Sharing on social media
- 📝 Writing a blog post
- 💰 [Sponsoring development](https://github.com/sponsors/Blues24)

---

## 📈 Stats

![GitHub stars](https://img.shields.io/github/stars/Blues24/Blues-Zencore?style=social)
![GitHub forks](https://img.shields.io/github/forks/Blues24/Blues-Zencore?style=social)
![GitHub watchers](https://img.shields.io/github/watchers/Blues24/Blues-Zencore?style=social)

![GitHub issues](https://img.shields.io/github/issues/Blues24/Blues-Zencore)
![GitHub pull requests](https://img.shields.io/github/issues-pr/Blues24/Blues-Zencore)
![GitHub last commit](https://img.shields.io/github/last-commit/Blues24/Blues-Zencore)

---

<div align="center">

**Made with 🦀 Rust and ❤️ by Blues24**

[⬆ Back to Top](#-blues-zencore)

</div>
