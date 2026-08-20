# <div align="center">

<a href="https://github.com/rouhalamin/checksum">

<img src="./assets/checksum-title.gif" alt="CheckSum — Secure File Integrity Verification" width="820">

</a>

<br>

### 🔐 Verify. Trust. Secure.

**A fast, lightweight and security-focused file integrity verification tool built with Rust.**

<br>

[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?style=for-the-badge\&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-blue?style=for-the-badge\&logo=windows)](https://www.microsoft.com/windows/)
[![License](https://img.shields.io/badge/License-GPL--3.0-red?style=for-the-badge)](./LICENSE)
[![Release](https://img.shields.io/github/v/release/rouhalamin/checksum?style=for-the-badge\&color=success)](https://github.com/rouhalamin/checksum/releases)
[![Repository](https://img.shields.io/badge/GitHub-Repository-black?style=for-the-badge\&logo=github)](https://github.com/rouhalamin/checksum)

</div>

---

## 🛡️ What is CheckSum?

**CheckSum** is a security-focused file integrity verification tool designed to help users determine whether a downloaded file matches a trusted cryptographic hash.

Downloading software from torrents, mirrors, file-sharing platforms, unofficial websites, or other untrusted sources can expose users to tampered or modified files.

CheckSum provides a simple verification workflow:

> **Choose the file → Enter the trusted SHA-256 hash → Verify → Trust or Reject**

If the calculated SHA-256 hash matches the trusted reference hash, the file contents match that reference.

If the hashes do **not** match, the file is different from the trusted reference and should **not** be considered authentic.

### Why it matters

A downloaded file can look completely normal while its contents have been modified.

Checksum verification gives you a cryptographic way to compare the file you received against a known-good hash published by the developer or another trusted source.

---

## ⚡ Why CheckSum?

* 🦀 **Built with Rust** — designed around performance, reliability and Rust's memory-safety guarantees.
* 🔐 **SHA-256 verification** — uses a widely adopted cryptographic hash function for file integrity verification.
* 🚀 **Fast and lightweight** — focused on doing one security task efficiently.
* 🖥️ **Windows focused** — supports Windows 10 and Windows 11.
* 🎯 **Simple workflow** — select a file, provide its trusted hash, and verify.
* 🔎 **Security-first purpose** — built specifically to help users detect modified or corrupted downloads.
* 🆓 **Open Source** — released under the GPL-3.0 license.

---

# 🚀 Download

<div align="center">

<a href="https://github.com/rouhalamin/checksum/releases/download/v1.1.0/CheckSum.exe">
<img src="https://img.shields.io/badge/⬇%20Download%20CheckSum%20v1.1.0-00C853?style=for-the-badge&logo=windows&logoColor=white" alt="Download CheckSum v1.1.0">
</a>

  

<a href="https://checksumapp.netlify.app/">
<img src="https://img.shields.io/badge/🌐%20Official%20Website-111827?style=for-the-badge&logo=netlify&logoColor=white" alt="Official Website">
</a>

</div>

### Current Release

**Version:** `v1.1.0`
**Executable:** `CheckSum.exe`
**Platform:** Windows 10 / 11

🔗 **GitHub Release:**
https://github.com/rouhalamin/checksum/releases/tag/v1.1.0

🔗 **Direct Download:**
https://github.com/rouhalamin/checksum/releases/download/v1.1.0/CheckSum.exe

🔗 **Official Website:**
https://checksumapp.netlify.app/

---

# 🔐 Verify the Download Before Running

For security software, **verification should come before execution**.

The SHA-256 hash published for the `CheckSum.exe` release is:

```text
bb42d17310d5e1662b2235821a5f248a77c2bcc48566dbef441e8b5feb3d24bf
```

### Windows built-in verification

Windows includes the `certutil` command, so no additional hashing software is required.

Open **Command Prompt** in the folder containing `CheckSum.exe` and run:

```cmd
certutil -hashfile CheckSum.exe SHA256
```

The resulting hash should be:

```text
bb42d17310d5e1662b2235821a5f248a77c2bcc48566dbef441e8b5feb3d24bf
```

### ✅ Matching hash

If the calculated SHA-256 value exactly matches the published reference:

```text
bb42d17310d5e1662b2235821a5f248a77c2bcc48566dbef441e8b5feb3d24bf
```

the downloaded executable matches the published reference hash.

### ❌ Different hash

If even **one character** differs, the file does not match the published reference.

Do **not** assume that the file is authentic. Download the executable again from the official release page and verify it again.

---

# 🪟 Windows SmartScreen

Because CheckSum is distributed as an independently developed Windows executable and may not have a commercial code-signing certificate, Microsoft Defender SmartScreen may display a warning such as:

> **Windows protected your PC**

This warning does **not by itself prove that the program is malicious**. However, it also should not simply be ignored.

### Recommended verification process

Before choosing to continue:

1. Download `CheckSum.exe` from the official GitHub Release.
2. Calculate its SHA-256 hash with:

```cmd
certutil -hashfile CheckSum.exe SHA256
```

3. Compare the result against the published hash in this README.
4. Only continue when the hash matches the trusted reference.
5. If SmartScreen still prevents execution and you have independently verified the hash and source, Windows may provide **More info → Run anyway**.

> ⚠️ **Security note:** Never bypass SmartScreen merely because a file refuses to run. Verify the source and SHA-256 hash first.

---

# 🧭 How It Works

CheckSum follows a straightforward integrity-verification model:

```text
                 ┌─────────────────────┐
                 │   Download a file   │
                 └──────────┬──────────┘
                            │
                            ▼
                 ┌─────────────────────┐
                 │     Open CheckSum   │
                 └──────────┬──────────┘
                            │
                            ▼
                 ┌─────────────────────┐
                 │    Select the file  │
                 └──────────┬──────────┘
                            │
                            ▼
                 ┌─────────────────────┐
                 │ Enter trusted hash  │
                 └──────────┬──────────┘
                            │
                            ▼
                  ┌───────────────────┐
                  │ Calculate SHA-256 │
                  └─────────┬─────────┘
                            │
                   ┌────────┴────────┐
                   ▼                 ▼
              ✅ MATCH             ❌ MISMATCH
                   │                 │
                   ▼                 ▼
               Trusted          Do not trust
               reference        the file
               match
```

---

# 🧪 Example

Suppose a developer publishes the following SHA-256 value for a program:

```text
bb42d17310d5e1662b2235821a5f248a77c2bcc48566dbef441e8b5feb3d24bf
```

You download the program and calculate its SHA-256 locally:

```cmd
certutil -hashfile CheckSum.exe SHA256
```

If Windows returns:

```text
bb42d17310d5e1662b2235821a5f248a77c2bcc48566dbef441e8b5feb3d24bf
```

the downloaded file matches the published reference.

If Windows returns anything else, the file is not identical to the published reference.

---

# 🔏 Important Security Concept

A SHA-256 comparison answers one specific question:

> **Does this file produce the same SHA-256 digest as the trusted reference?**

It does **not** automatically prove that the publisher is trustworthy.

For meaningful verification, always obtain the reference hash from a source you trust, such as:

* the official project repository;
* the official release page;
* a trusted developer publication channel;
* or another authenticated distribution channel.

**Never trust a hash that was supplied by the same untrusted source as the file you are attempting to verify.**

---

# 💻 Supported Platform

| Platform   | Status                    |
| ---------- | ------------------------- |
| Windows 10 | ✅ Supported               |
| Windows 11 | ✅ Supported               |
| Linux      | 🚧 Not currently targeted |
| macOS      | 🚧 Not currently targeted |

---

# 🦀 Technology

CheckSum is written in **Rust**.

The project uses Rust because the language provides strong compile-time safety guarantees while allowing developers to build high-performance native applications.

### Core technology

```text
Language       → Rust
Integrity      → SHA-256
Platform       → Windows 10 / 11
Distribution   → GitHub Releases
License        → GPL-3.0
```

---

# 📦 Source Code

The complete source code is publicly available:

**GitHub Repository**
https://github.com/rouhalamin/checksum

The project is intended to remain transparent and auditable so users and security professionals can inspect the implementation themselves.

---

# ❤️ Support Independent Open-Source Development

<div align="center">

## Help Keep CheckSum Alive

</div>

Hi, I'm **Rouhalamin**.

I spent months building this CheckSum tool from scratch using Rust, pouring my heart and countless sleepless nights into optimizing its performance, reliability, and security.

In a world filled with similar applications, I wanted to create something exceptionally fast, focused, and trustworthy.

But I'm an independent developer without financial backing. Competing with companies that have large budgets, teams, infrastructure, and commercial security resources is a massive challenge.

I refused to put this tool behind a paywall because I believe that **everyone deserves access to basic security tools.**

If CheckSum protects your files, helps you verify a download, saves you time, or simply gives you peace of mind, your support can help keep this project alive.

Every contribution — large or small — can help fund future development, testing, infrastructure, documentation, security improvements, and new features.

### Thank you for supporting independent open-source development. ❤️

---

## 💰 Cryptocurrency Donations

<div align="center">

|                          🟠 Bitcoin (BTC)                         |                          ♦️ Ethereum (ETH)                         |                            🟢 Tether (USDT)                           |
| :---------------------------------------------------------------: | :----------------------------------------------------------------: | :-------------------------------------------------------------------: |
| <img src="./assets/btc-qr.png" alt="Bitcoin QR Code" width="180"> | <img src="./assets/eth-qr.png" alt="Ethereum QR Code" width="180"> | <img src="./assets/usdt-qr.png" alt="USDT TRC20 QR Code" width="180"> |
|                        **Network:** Bitcoin                       |                **Network:** Base / Optimism / ERC-20               |                       **Network:** TRC-20 (TRON)                      |
|            `bc1qgsxsy5wp6twxhuxhvnujd8pza788ra5j4j67gu`           |            `0x6E6764da4c477a08318224B992Bb54Cedd4197a0`            |                  `TDcg8aKadKRZx97Fw5p7BfCGXBBgWFy6Vh`                 |

</div>

### ⚠️ Donation Safety

Always verify the wallet address before sending funds.

For Ethereum:

```text
Base       → 0x6E6764da4c477a08318224B992Bb54Cedd4197a0
Optimism   → 0x6E6764da4c477a08318224B992Bb54Cedd4197a0
ERC-20     → 0x6E6764da4c477a08318224B992Bb54Cedd4197a0
```

For USDT:

```text
TRC-20 / TRON → TDcg8aKadKRZx97Fw5p7BfCGXBBgWFy6Vh
```

> ⚠️ **Always select the correct blockchain network when sending cryptocurrency.**

---

# 🛡️ Responsible Disclosure

Security is a core part of CheckSum.

If you discover a security vulnerability, implementation flaw, or other issue that could affect the security or integrity of the project, please report it responsibly.

### Please do not publicly disclose an exploitable vulnerability before giving the maintainer reasonable time to investigate and address it.

Send security reports directly to:

**📧 [rouhalaminerfani@gmail.com](mailto:rouhalaminerfani@gmail.com)**

When reporting a vulnerability, please include:

* a clear description of the issue;
* affected version(s);
* reproduction steps;
* proof-of-concept details where appropriate;
* potential security impact;
* and any suggested mitigation.

Security researchers who responsibly report valid issues are greatly appreciated. 🔐

---

# 🐛 Bug Reports & Feature Requests

Found a bug that is not security-sensitive?

Please open a GitHub Issue:

https://github.com/rouhalamin/checksum/issues

When creating an issue, include as much useful information as possible:

```text
CheckSum Version:
Windows Version:
Expected Behavior:
Actual Behavior:
Steps to Reproduce:
Additional Information:
```

For sensitive security vulnerabilities, use the private security email instead of creating a public issue.

---

# 🤝 Contributing

Contributions are welcome.

You can contribute by:

* reporting bugs;
* improving documentation;
* reviewing source code;
* suggesting features;
* submitting pull requests;
* testing new releases;
* or helping improve the security of the project.

Repository:

https://github.com/rouhalamin/checksum

Before submitting a pull request, please make sure your changes are clear, tested where applicable, and consistent with the project's existing architecture.

---

# 📜 License

CheckSum is licensed under the:

## GNU General Public License v3.0 — GPL-3.0

The GPL-3.0 license provides users with the freedom to use, study, modify, and redistribute the software under its terms.

See the full license:

[LICENSE](./LICENSE)

---

# 👤 Developer

<div align="center">

### Built and maintained by **Rohulamin Erfani**

Security-focused software developer and independent open-source creator.

<br>

[![GitHub](https://img.shields.io/badge/GitHub-rouhalamin-181717?style=for-the-badge\&logo=github)](https://github.com/rouhalamin)
[![LinkedIn](https://img.shields.io/badge/LinkedIn-Rohulamin%20Erfani-0A66C2?style=for-the-badge\&logo=linkedin)](https://www.linkedin.com/in/rouhalamin/)
[![Email](https://img.shields.io/badge/Email-Contact-D14836?style=for-the-badge\&logo=gmail\&logoColor=white)](mailto:rouhalaminerfani@gmail.com)

</div>

---

# 🌐 Connect With Me

<div align="center">

<a href="https://wa.me/93702302034">
<img src="https://img.shields.io/badge/WhatsApp-25D366?style=for-the-badge&logo=whatsapp&logoColor=white" alt="WhatsApp">
</a>

<a href="https://t.me/rouhalamin_erfani">
<img src="https://img.shields.io/badge/Telegram-26A5E4?style=for-the-badge&logo=telegram&logoColor=white" alt="Telegram">
</a>

<a href="https://www.linkedin.com/in/rouhalamin/">
<img src="https://img.shields.io/badge/LinkedIn-0A66C2?style=for-the-badge&logo=linkedin&logoColor=white" alt="LinkedIn">
</a>

<a href="https://github.com/rouhalamin">
<img src="https://img.shields.io/badge/GitHub-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub">
</a>

<a href="mailto:rouhalaminerfani@gmail.com">
<img src="https://img.shields.io/badge/Email-D14836?style=for-the-badge&logo=gmail&logoColor=white" alt="Email">
</a>

<a href="https://www.instagram.com/rouhalaminerfani/">
<img src="https://img.shields.io/badge/Instagram-E4405F?style=for-the-badge&logo=instagram&logoColor=white" alt="Instagram">
</a>

</div>

---

# 🌐 Official Links

| Resource                    | Link                                                                         |
| --------------------------- | ---------------------------------------------------------------------------- |
| 📦 GitHub Repository        | https://github.com/rouhalamin/checksum                                       |
| 🚀 Latest Release           | https://github.com/rouhalamin/checksum/releases/tag/v1.1.0                   |
| ⬇️ Download CheckSum v1.1.0 | https://github.com/rouhalamin/checksum/releases/download/v1.1.0/CheckSum.exe |
| 🌐 Official Website         | https://checksumapp.netlify.app/                                             |
| 🐛 Issues                   | https://github.com/rouhalamin/checksum/issues                                |
| 📧 Security Contact         | mailto:rouhalaminerfani@gmail.com                                            |

---

# 🔎 Verification Reference

For **CheckSum v1.1.0**:

```text
File:
CheckSum.exe

SHA-256:
bb42d17310d5e1662b2235821a5f248a77c2bcc48566dbef441e8b5feb3d24bf
```

Windows command:

```cmd
certutil -hashfile CheckSum.exe SHA256
```

---

<div align="center">

## 🔐 Verify Before You Trust.

### Built with Rust. Designed for integrity. Open source by choice.

<br>

**CheckSum — Protecting trust, one file at a time.**

<br>

⭐ If CheckSum is useful to you, consider giving the repository a star.

<br>

© Rohulamin Erfani — CheckSum
Licensed under GPL-3.0

</div>
