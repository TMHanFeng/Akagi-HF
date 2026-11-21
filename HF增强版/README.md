# Akagi-HF · 增强版

这是 [shinkuan/Akagi](https://github.com/shinkuan/Akagi) **v3 分支**的增强版。
上游历史完整保留 —— `v3` 分支上多出来的那**一个提交**就是全部改动（31 个文件）。

## 相对上游 v3 多了什么

| 改动 | 说明 | 详见 |
|---|---|---|
| **续局模块（requeue）** | 一局结束后自动走完结算流程，点「再來一場」开下一局 | 说明书 §4 |
| **运行时可调的打牌计划** | 悬浮窗里加一行：已打/还剩、「这局后停」、每天的截止时刻 | 说明书 §4.11 |
| **随机延迟打牌** | 在思考时间模型之上叠一层随机延迟（碰/吃除外） | 说明书 §3 |
| **模型自由选择** | 设置页里能选模型、能拉模型列表 | 说明书 §2 |
| **后台运行** | 三条启动参数让浏览器被遮挡 / 移出屏幕 / 待在非活动虚拟桌面时也保持 60fps | 说明书 §5 |
| **identifier 改名** | `com.akagi.akagi` → `com.akagi.akagi.v3`，和 AkagiMS 并存不打架 | 说明书 §7 |

## 本目录

```
HF增强版/
  Akagi-v3增强版使用说明.md   # 完整说明书（11 章，所有实测数据都在里面）
  Akagi-v3增强补丁.patch      # 对上游 v3 的完整补丁（31 个文件，git apply 可直接应用）
  校准工具/                   # 坐标校准脚本（说明书 §4.9 会用到）
```

编译好的 `akagi.exe` 和打包好的 zip **不在这个仓库里** —— 二进制不该进 git。
用上面的补丁自己构建，或者从发布页取成品。

## 自己构建

见说明书的 §8。需要 Rust 1.100+、Node 20+、Python 3.10+。

```bash
git clone https://github.com/TMHanFeng/Akagi-HF.git
cd Akagi-HF
git apply HF增强版/Akagi-v3增强补丁.patch   # 只想要差异时用
cd frontend && npm ci && npm run build && cd ..
cargo build --release --features custom-protocol
```

## 已验证

Rust 单测 **1108/1108**、前端单测 **109/109**，`tsc -b` / `eslint` / `vite build` 全过；
补丁经 `git apply` 后与工作树 **31/31 逐字节一致**。

具体的实机验证记录（19 局无人值守续局、后台 60fps、悬浮窗计划行等）都在说明书 §9。
