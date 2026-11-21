# Akagi v3 增强版 · Windows 构建与运行指南

> 本仓库是 [shinkuan/Akagi](https://github.com/shinkuan/Akagi) `v3` 分支的 fork（增强版）。
> **仓库里放的是源码，不是安装包。** 想直接用 → 看第 1 节；想自己改 → 看第 2 节。

| | 是什么 | 在哪 |
|---|---|---|
| **本仓库** | Rust(Tauri) 后端 + React 前端的**源码** | 克隆下来是 `src/`、`frontend/`、`Cargo.toml`、`build.rs` … |
| **成品包** | 编译好的 `akagi.exe` + 校准工具 + 使用说明 | [Releases 页](../../releases) 下载 `Akagi-v3增强版.zip` |

上游 README（[English](../../README.md) / [简体中文](../../README.zh-CN.md) / [繁體中文](../../README.zh-TW.md)）
讲的是通用上游用法；本文件讲的是**这个 fork 在 Windows 上真实跑通过的那条路**。

---

## 1. 路线 A：下载成品直接跑（推荐）

1. 打开 [Releases 页](../../releases)，下载 `Akagi-v3增强版.zip`
2. 解压到一个**固定目录**，例如 `C:\Users\<你的用户名>\Desktop\Akagi-v3增强版\`
   - 每个实例一个独立文件夹（两个实例共用同一目录会抢 `chrome-profile`）
3. **先关掉 AkagiMS**，再双击 `akagi\akagi.exe`
   - 原因见 [第 5 节](#5-为什么必须先关-akagims)
4. 首次启动：配置向导里抓包模式选 **Chromium** → 扫码登录
5. 打开「自动下一局」开关 → 开始打牌

解压后的目录：

```
Akagi-v3增强版/
  akagi/
    akagi.exe               主程序 —— 双击这个
    WebView2Loader.dll      必须和 exe 同目录
    configs/                首次运行自动生成（config.toml 在这里）
    logs/  history/  ca/    同上
  校准工具/                  坐标校准脚本（需要 Python）
  Akagi-v3增强版使用说明.md   完整说明书（怎么配、每个旋钮什么意思）
  Akagi-v3增强补丁.patch     31 个文件的补丁，`git apply` 可直接用
```

完整的使用说明在压缩包里的 `Akagi-v3增强版使用说明.md`，比本文件详细得多。

---

## 2. 路线 B：从源码编译

### 2.1 前置依赖（Windows）

| 依赖 | 版本 | 说明 |
|---|---|---|
| Rust | stable ≥ 1.80，**必须装 `x86_64-pc-windows-gnullvm` 这个 host** | 见 2.2 ① |
| Node.js | 20+（本机实测 24.15.0 / npm 11.12.1） | 构建前端 |
| LLVM-MinGW | 20261117 msvcrt | 提供 `clang` / `llvm-ar` / `llvm-rc` |
| protoc | 近期版本 | `build.rs` 要用它编译 `liqi.proto` |
| cmake、nasm | 近期版本 | `aws-lc-sys`（rustls 的依赖）需要 |
| WebView2 | Win11 自带即可 | 运行期依赖 |

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnullvm
rustup default stable-x86_64-pc-windows-gnullvm
```

### 2.2 三个必踩的坑（照做即可）

**① 链接器：不要用 `x86_64-pc-windows-gnu`**

Rust 的 `x86_64-pc-windows-gnu` 目标会链接 `-lgcc -lgcc_eh -lmingwex -lmingw32` —— 那是
**GCC 运行时**的库。LLVM-MinGW 只有 `libunwind`，没有 GCC 运行时，直接链接会失败：

```
lld: error: unable to find library -lgcc_eh
```

解法是 **`x86_64-pc-windows-gnullvm`** —— Rust 官方把它做成了一等 host
（`rust-1.100.0-x86_64-pc-windows-gnullvm`），它的 std 链接的正是 `libunwind` + MSVC CRT，
和 LLVM-MinGW 完全对得上。

**② `tauri-build` 2.5.6 算错 target 目录**

它按固定层级从 `OUT_DIR` 反推 cargo 的 target 目录，而 cargo 后来把 `OUT_DIR` 从
`<pkg>-<hash>/out` 改成了 `<pkg>/<hash>/out`，于是它算错一层，Windows 分支去读一个
不存在的目录，build script 以 `系统找不到指定的路径 (os error 3)` 失败。

上游在 `tauri-build` 2.7.0 修了，但 2.7.x 要求 `tauri-utils ~2.10`，而本项目在 2.5 线上
（capability/permission schema 不通用），所以把上游那几行改法打到本地 registry 里的
2.5.6 副本上：

```bash
python - <<'PY'
import io, os
REG = os.path.expanduser("~/.cargo/registry/src")
OLD = """  let target_dir = out_dir
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .parent()
    .unwrap();
"""
NEW = """  let target_dir = out_dir
    .ancestors()
    .find(|path| path.file_name().map(|name| name == "build").unwrap_or(false))
    .and_then(|build_dir| build_dir.parent())
    .unwrap();
"""
for root, dirs, files in os.walk(REG):
    if "tauri-build-2.5.6" in dirs:
        path = os.path.join(root, "tauri-build-2.5.6", "src", "lib.rs")
        if os.path.exists(path):
            src = io.open(path, encoding="utf-8", newline="").read()
            if ".ancestors()" not in src and OLD in src:
                io.open(path, "w", encoding="utf-8", newline="").write(src.replace(OLD, NEW, 1))
                print("patched:", path)
            else:
                print("already patched or unexpected:", path)
            raise SystemExit(0)
raise SystemExit("tauri-build-2.5.6 not found under " + REG)
PY
```

> cargo 不跟踪 registry 源的 mtime，所以打完补丁**必须**把旧的 build 产物删掉，否则不会重编：
> ```bash
> rm -rf target/release/build/tauri-build target/$TARGET/release/build/tauri-build
> ```

**③ crates.io 索引太慢**

直连只有 ~3 MB/分，700 多个 crate 要两小时。改用 USTC 镜像，写进
`%USERPROFILE%\.cargo\config.toml`：

```toml
[source.crates-io]
replace-with = "ustc"

[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"

[net]
retry = 5
```

### 2.3 编译步骤

```bash
git clone https://github.com/TMHanFeng/Akagi-HF.git
cd Akagi-HF
git checkout v3

# --- 环境变量 ---
export RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnullvm
export TARGET=x86_64-pc-windows-gnullvm
export PATH="/c/tools/llvm-mingw/bin:/c/tools/protoc/bin:/c/tools/cmake/bin:$PATH"
export PROTOC="C:/tools/protoc/bin/protoc.exe"

export CC=clang
export CXX=clang++
export AR=llvm-ar
export RC=llvm-rc
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=clang
export LLVM_AR="C:/tools/llvm-mingw/bin/llvm-ar"
export LLVM_RANLIB="C:/tools/llvm-mingw/bin/llvm-ranlib"

# --- 前端 ---
cd frontend
npm ci
npm run build
cd ..

# --- 后端 ---
cargo build --release --features custom-protocol --target $TARGET
```

> **不要用 `cargo tauri build`。** 它的 `beforeBuildCommand` 会跑 `npm ci`，而 `npm ci` 会清空
> `node_modules` —— 里面包含**正在运行的 Tauri CLI 可执行文件**，文件被锁导致构建失败。
> 前端已经手动 `npm run build` 过了，用普通 `cargo build` 即可。
>
> `--features custom-protocol` 不能少：少了它 Tauri 会以 dev 模式构建，窗口去连
> `localhost:1420`，表现为「localhost 拒绝连接」。

产物：

```
target/x86_64-pc-windows-gnullvm/release/akagi.exe
```

把 `WebView2Loader.dll`（WebView2 的动态加载器，**必须和 exe 同目录**）一起拷过去，
双击即可运行。首次运行会在 exe 旁边生成 `configs/`、`logs/`、`history/`、`ca/`。

### 2.4 跑测试

```bash
cargo test                                    # 全量单测 + 集成测试
cd frontend && npx vitest run && npx tsc -b    # 前端
```

---

## 3. 这个增强版改了什么

相对上游 `v3`，改动集中在 31 个文件（清单见压缩包里的说明书 §6，补丁见
`Akagi-v3增强补丁.patch`）：

- **自动下一局（requeue）** —— 结算后自动点「再來一場」并处理后续对话框；走屏用固定
  顺序 + 盲点对话框
- **打牌计划（运行时可调）** —— 悬浮窗底部一行状态条，点「≡」展开面板，可设
  「还剩几局」「+N 天」「截止时刻」「这局后停」；截止时刻是**绝对时刻**
  （`YYYY-MM-DDTHH:MM`），到点把当前这局打完就停，不再开下一局
- **随机延迟出牌** —— 可配延迟区间，「碰/吃」除外，并按服务器预算卡上限
- **模型自由选择** —— 前端下拉拉取模型列表，不再写死
- **`--disable-features` 合并** —— 修掉「最后一条参数无效」这个真 bug
- **identifier 改成 `com.akagi.akagi.v3`** —— 见第 5 节

---

## 4. 回到纯净上游

```bash
git remote add upstream https://github.com/shinkuan/Akagi.git
git fetch upstream
git diff upstream/v3..v3 --stat   # 看全部改动
git log  upstream/v3..v3          # 看提交历史
```

或直接用 `Akagi-v3增强补丁.patch` 反向还原：

```bash
git apply -R Akagi-v3增强补丁.patch
```

---

## 5. 为什么必须先关 AkagiMS

上游 v3 的 Tauri identifier 是 `com.akagi.akagi`，**和 AkagiMS 一样**。WebView2 的浏览器进程
按**用户数据目录**（`%LOCALAPPDATA%\<identifier>\EBWebView`）共享，两个应用 identifier 相同
就会共用同一个浏览器进程；而 MS 文档写明：

> As a browser process may be shared among WebViews, WebView creation fails with
> `HRESULT_FROM_WIN32(ERROR_INVALID_STATE)` if the specified options does not
> match the options of the WebViews that are currently running in the shared
> browser process.

实测就是：AkagiMS 开着时启动本构建，日志里两个 webview 全部
`failed to create webview: HRESULT(0x8007139F)`，**窗口根本出不来**；关掉 AkagiMS 再启动，
立刻正常。

所以补丁把 identifier 改成 `com.akagi.akagi.v3`，让两个应用各用各的 WebView2 数据目录。
**改完之后实测：AkagiMS 和本构建同时运行，两个窗口都正常。**

---

## 6. 常见问题

**Q：`lld: error: unable to find library -lgcc_eh`**
→ 用了 `x86_64-pc-windows-gnu`。换 `x86_64-pc-windows-gnullvm`（见 2.2 ①）。

**Q：build script 报 `系统找不到指定的路径 (os error 3)`**
→ `tauri-build` 没打补丁，见 2.2 ②（打补丁后记得删 `target/**/build/tauri-build`）。

**Q：窗口显示「localhost 拒绝连接」**
→ 漏了 `--features custom-protocol`，或者前端没 `npm run build`。

**Q：打包后启动就闪退 / 出不来窗口**
→ 先关 AkagiMS（第 5 节）。

**Q：改了 `configs/config.toml` 但被覆盖**
→ 退出程序时 v3 会用内存里的旧值写回 `config.toml`，所以改配置前先退出程序。

**Q：两个实例怎么跑**
→ 每个实例一个独立文件夹（含 `chrome-profile`），**不要复制 `chrome-profile`**。

---

## 7. 许可与出处

- 上游：[shinkuan/Akagi](https://github.com/shinkuan/Akagi)，Apache-2.0
- 本 fork 保留上游的 `LICENSE.txt` / `NOTICE` / `README.*.md`
- 内置 Mortal 引擎为 AGPL-licensed，通过 JSONL 进程边界与主程序交互（不构成衍生作品）
