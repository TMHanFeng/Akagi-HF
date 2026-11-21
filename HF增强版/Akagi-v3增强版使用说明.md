# Akagi v3 增强版使用说明

在开源的 [shinkuan/Akagi](https://github.com/shinkuan/Akagi) v3（Apache-2.0）上加了三个能力：

1. **模型自由选择** —— 4 人 / 3 人分别用哪个模型，在界面里点一下就能换，打到一半换也立刻生效。
2. **随机延迟打牌** —— 每次决策额外叠一段随机等待（范围可配、**碰/吃除外**、可开关）。
3. **续局模块（requeue）** —— 一局结束后自动点掉结算画面、点大厅「再来一局」、计数、到局数自动停。

外加一件必须先说清楚的事：**Mortal 权重到底能不能下载**（§1，全部是实测数据，不是推测）。

> 所有参数都在**前端设置界面**里改，不用手改配置文件。

---


---

## 快速开始：三步跑起来

### ① 解压 → **先关掉 AkagiMS** → 双击

把 `Akagi-v3增强版.zip` 解压到任意**有写权限**的目录（桌面也行）。
`akagi.exe` 和 `WebView2Loader.dll` **必须在同一个文件夹**（包里已经放好了）。

> ⚠️ **先把 AkagiMS 关掉，再开这个。**
> 同一个雀魂账号**不能同时在两个客户端登录**，会互相顶掉。
> 我们的补丁把 Tauri identifier 改成了 `com.akagi.akagi.v3`（§7），
> 所以两个 exe **可以同时开着、窗口不冲突** —— 但**账号只能在一处登录**。

SmartScreen 会拦一下 → *More info → Run anyway*。
需要 WebView2 Runtime（Win10 1803+ / Win11 已预装，本机已装 157.0.4341.60）。

### ② 首次启动：走配置向导，抓包模式选 **Chromium**

双击 `akagi.exe`，向导依次问：**语言 → 平台（选「雀魂」）→ 抓包模式 → bot 配置**。

* **抓包模式选「Chromium」** —— 最省事的一条：不用装证书、不用配代理。
  Akagi 会自己找 Chrome / Edge / Brave / Chromium，用**独立的用户配置**把它们拉起来。
  （选 MITM 模式才需要信任 `./ca/akagi-ca.crt` 并配 Proxifier，步骤见上游 README。）
* **bot 不用装**：内置 bot（`akagi-native`）的权重是**编译进 exe** 的，开箱即用。

### ③ 扫码登录 → 打开「自动下一局」开关

Akagi 会拉起一个浏览器窗口打开雀魂，在里面**扫码登录**。登录后回到主界面：

**设置 → 自动打牌 → 拉到最下面 → 打开「一局结束后自动开始下一局」开关。**

四个坐标（確定 / 再來一場 / 对话框的「確定」/ 匹配對局）**默认已经是实测值，不用改**。
「局数上限」默认 0 = 不限。

### ④ 想让它待在后台？加三行启动参数

**默认状态下浏览器窗口必须待在前台**（切到别的程序但窗口还完整可见是可以的）。
想把它丢到屏幕外、或者压在别的软件底下继续打，
去 **设置 → 抓包模式 → Chromium → 额外启动参数（extra_args）** 填上：

```
--disable-backgrounding-occluded-windows
--disable-renderer-backgrounding
--disable-features=CalculateNativeWinOcclusion
```

填完**重启 v3**。原因、实测帧率表和「为什么最小化没救」见 **§5**。

### 第一次用建议盯着日志

`akagi.exe` 同目录下会有 `logs/`，每次会话一个目录。续局模块每一步都会打一行：

```
requeue: game 1 ended — walking the result screens
requeue: the match request went out after 6 press(es)
```

看到 `the match request went out` 就是这一局接上了；
看到 `the press budget ran out` 就是没接上 —— 回去看 §4.9 重新校准坐标。

### 常见问题

| 现象 | 原因 / 怎么办 |
|---|---|
| 登录后马上掉线 | 另一个客户端还开着 —— 同一个账号只能一处登录，关掉 AkagiMS |
| 窗口开了点不动 / 白屏 | 先关 AkagiMS 再开（identifier 已改，一般不会撞，但保险起见） |
| 打完一局不自动接下一局 | 开关没打开；看 `logs/` 里的 `requeue:` 行；坐标失效风险见 §4.10 |
| 窗口切到后台就不动了 | 需要按 §5 配那三行 `extra_args` 并重启；**最小化一定不行**，别的都行了 |
| 想用 Mortal 之类的 **Python bot** 而不是内置 bot | 我们这个包**不含** Python 运行时（那在官方 254 MB Release 里）。把官方包里的 `runtime/python/x86_64-pc-windows-msvc` 和 `runtime/uv/x86_64-pc-windows-msvc` 复制到 `akagi.exe` 旁边，并**改名为 `x86_64-pc-windows-gnullvm`** —— 我们的构建是 gnullvm 目标，代码按这个 triple 找运行时 |

---

## 1. Mortal 权重下载方法（结论先行）

**结论：雀魂用的强 Mortal v4 权重，没有公开下载源。**
能公开下载到的要么是**占位权重**，要么是**日麻权重**，雀魂都用不了。
现在唯一能真正用上强 v4 的公开途径是**云端推理 API（需要 key）** —— 而本增强版把「选哪个模型」做成了界面里点一下就能切。

### 1.1 全部可下载源的实测结果

| 来源 | 直链 | 实测体积 | 能不能用 | 证据 |
|---|---|---|---|---|
| `shinkuan/Akagi-MjaiBot-Mortal` 仓库根目录的 `mortal.pth` | `https://raw.githubusercontent.com/shinkuan/Akagi-MjaiBot-Mortal/main/mortal.pth` | **5,123,618 B** | ❌ 占位 | 下载后用 torch 读出 `config.resnet.conv_channels = 32`、`config.resnet.num_blocks = 2`、`config.control.version = 4`；42 个张量、**1,228,649 参数** |
| 同仓库的 release 包 | `https://github.com/shinkuan/Akagi-MjaiBot-Mortal/releases/latest/download/release4p.zip` | **31,370,577 B** | ❌ 同上 | GitHub API 的仓库树里 `mortal.pth` 就是 5,123,618 B；这个 zip = 占位权重 + libriichi 扩展 |
| `lynkas/c3` 的 `model-c3-best.pth` | `https://github.com/lynkas/c3/releases/download/v1.0/model-c3-best.pth` | **138,731,342 B** | ❌ 是**日麻**权重 | 该项目是 libriichi / 日麻推理栈，雀魂引擎吃不了；且均位 2.492 弱于 Mortal v4 的 2.419 |
| 云端 MJAPI | `https://mjapi.shinkuan.me` | — | ✅ **能** | `GET /healthz` → `{"status":"ok","queue_depth":1,"workers_alive":true}`；`GET /v3/models` 需要 key（无 key 返回 **403**） |
| Mortal bot 自带的在线推理端点 | `http://server.akagiot.org/` | — | ⚠️ 本机被 Cloudflare 挡 | 四个 URL（`/`、`/react_batch`、`/healthz`、https 版）全部返回 **403**，返回的是 Cloudflare 拦截页；这个端点要该 bot 设置里的 `api_key` |

> 大陆网络提示：GitHub Release 直连极慢，走 `https://gh-proxy.com/` 前缀可下（支持 Range，断点续传有效）。

### 1.2 为什么说那 5 MB 的是「占位」而不是训练好的 v4

生产 Mortal v4 = **192 通道 / 40 残差块**（服务端给模型写的描述就是这个：
`"4p Mortal v4 (ot2), 192x40"`，见 v3 `src/bot/api.rs` 里 `/v3/models` 的样例数据）。

公开可下的那份权重，实际读出来是：

```
config.control.version      = 4        # 架构版本确实是 v4
config.resnet.conv_channels  = 32       # 生产是 192 → 宽度差 6 倍
config.resnet.num_blocks     = 2        # 生产是 40  → 深度差 20 倍
state_dict 张量数            = 42
参数量                       = 1,228,649
```

也就是**同一套架构的缩小版**，不是训练好的 v4。

> **顺带一个反直觉的事实：v3 自带的本地模型也很小。**
> `native_bot/weights/akagi4p.safetensors` 只有 **2,641,816 B**，
> safetensors 头里写着 `conv: 64, blocks: 3` —— 64 通道 / 3 块。
> 比那个「占位 Mortal」大一点，但离生产的 192×40 还差得很远。
> 所以「不配云端、直接用 v3 内置本地模型」，打出来的水平是明显偏弱的。

### 1.3 真正的强权重在哪

1. **作者的 Discord**（上游安装向导把用户指向那里）—— 需要账号，本包无法替你核实。
2. **云端 MJAPI** —— **唯一一条你现在就能走通、且能直接用上强 v4 的路**。
   走 §2 的模型选择即可，不需要碰任何权重文件。

### 1.4 一句话建议

想要强 AI → **拿一个 MJAPI key，然后在设置里把四人模型选成 v4**；
不要指望从 GitHub release 里扒权重，扒到的都是占位。

---

## 2. 模型自由选择

### 2.1 界面在哪

**机器人（Bots）页最上面**的「MJOT 云端推理」卡片 → 「四人模型」「三人模型」两个下拉框。

- 打开界面时如果**已经存了 key 且云端推理是开的**，下拉框会自动列好这个 key 能用的模型（每个都带一句描述，例如 `4p-ot2 — 4p Mortal v4 (ot2), 192x40`）。
- 没存 key、或服务器连不上时，退回成**手填**的输入框，可以直接写模型 ID。
- 下拉里第一项是「**服务器默认**」—— 选它等于不指定模型，让服务器自己挑。
- 当前配置里的模型 ID **一定会出现在下拉里**，哪怕服务器已经不提供它（避免它被悄悄换成别的模型）。

### 2.2 生效时机

模型 ID 是**每次决策都被重新读的**（`bot.api` 这一节每次 react 都重读），
所以**不用重启，打到一半换模型也立刻生效**，日志里会打一条 `updated`。

### 2.3 配置文件

```toml
[bot.api]
enabled = true
base_url = "https://mjapi.shinkuan.me"
key = "…32 位…"
model_4p = "4p-ot2"     # ← 就是这里
model_3p = "3p-ot2"
```

### 2.4 如果你用别的推理端点

`bot.api.base_url` 指向任何兼容 `{base}/v3/react` + `{base}/v3/models` 的服务都行
（开发者模式才能编辑这个 URL）。模型下拉框用的是同一套 `/v3/models`，
所以换端点不需要改代码。

---

## 3. 随机延迟打牌

### 3.1 它加在哪里

加在**所有决策的最后一步**：模型算完思考时间之后，再叠一段随机等待才点。
也就是说它是「**在原有节奏上再抖一下**」，不是替换掉原有的延迟模型 ——
校准过的思考时间、按钮渲染等待、服务器时间预算全都还在。

### 3.2 界面

**设置 → 自动打牌** → 「随机延迟打牌」区块：

| 界面项 | 配置键 | 默认 | 说明 |
|---|---|---|---|
| 随机延迟打牌 | `enabled` | **关** | 总开关 |
| 随机延迟范围 (ms) | `min_ms` / `max_ms` | **0 – 3000** | 双滑块；每次决策在区间里**均匀随机**取一个值 |
| 碰/吃除外 | `exclude_chi_pon` | **开** | 碰和吃不叠加随机延迟 |

### 3.3 为什么默认「碰/吃除外」

碰和吃的**反应窗口很短**，晚一点就直接失去这次机会 —— 在那儿加延迟只有损失、没有收益。
吃和碰之外的动作（打牌、立直、荣和、自摸、跳过、北拔等）都会叠随机延迟。

### 3.4 预算兜底（不会撞上「时间到了自动出牌」）

随机值不是无脑加上去的：它被**服务器剩余时间**卡住 ——
`预算上限 − 模型目标` 还剩多少，就最多加多少。
所以就算你把范围拉到 10 秒，也不会把窗口拖到客户端自动弃牌。

### 3.5 配置文件

```toml
[autoplay.majsoul.random_delay]
enabled = true
min_ms = 0
max_ms = 3000
exclude_chi_pon = true
```

---

## 4. 续局模块（requeue）

### 4.1 它解决什么问题

AkagiMS（雀魂专用版）的「全自動」能自动续局，但它被一道**云端 key 门禁**挡住：

> Full auto needs a cloud key. Add one under Settings → Cloud inference, then pick full auto again.

实测 AkagiMS 日志表明，它的「自動」模式其实**已经自动点掉了结算画面**，
坐标固定在 16:9 归一化的 `(14.59, 8.37)`；但**从结算画面回到大厅、再点「再来一局」排队这一步，
它不做** —— 那是「全自動」模式的工作，而「全自動」要 key。

本模块把缺的那一步补上，并加上局数上限。

### 4.2 实测的结算流程与四个坐标

2026-11-20 在**繁体**客户端上实测（半庄结束后用系统级真实鼠标事件逐屏点过去，
同时开着 wire 日志逐帧记录），一局结束后的完整流程是：

1. **終局**积分板 → 右下角金色「確定」
2. **一姬的禮物盒**面板 → **同一个位置**的「確定」
3. **道具**奖励面板（浮汗藥水 +1 / 迷霧晶塵 +100）→ **同一个位置**的「確定」
   **+ 紧挨着它左边的蓝色「再來一場」**
4. 点「再來一場」**不会直接开局** —— 它弹出一个**二次确认框**
   （「要再進行一場　賣牌雙刃・四人東　の匹配對局嗎?」），
   框里左边是金色「確定」、右边是蓝色「取消」，右上角还有一个 X
5. 点那个框里的「確定」→ 客户端发出 `startUnifiedMatch` → **下一局开始**
6. **如果第 3 步改点「確定」而不是「再來一場」** → 客户端**不会回大厅**，
   而是进到**匹配准备画面**：一张「寶牌雙刃」的规则说明卡，
   下面一个橙色大按钮「**匹配對局**」→ 点它**直接开局**（没有确认框），
   客户端同样发出 `startUnifiedMatch`

于是是**四个**坐标（16:9 归一化，对**游戏 canvas**）：

| 按钮 | 归一化坐标 | 出现在哪 |
|---|---|---|
| 確定 | **(14.59, 8.37)** | 上面第 1~3 步的三个画面**共用**同一个坐标 |
| 再來一場 | **(12.23, 8.34)** | 只在第 3 步的道具奖励面板上，在「確定」左侧约 2.4 个归一化单位处 |
| 再來一場对话框里的「確定」 | **(6.58, 6.71)** | 只在第 4 步那个居中对话框里，在左边（右边是「取消」） |
| 匹配對局 | **(10.87, 7.10)** | 只在第 6 步的匹配准备画面上，橙色大按钮，居中偏下 |

> **第 4 步的对话框是上一轮实测才发现的**，之前一直以为「再來一場」按下去就直接开局。
> 实测证据：15:48:45.523 点「再來一場」→ 对话框弹出，**wire 上 0 帧**；
> 15:50:51.959 点对话框里的「確定」→ **15:50:54.018 `.lq.Lobby.startUnifiedMatch`**
> → 15:50:57.794 `NotifyMatchGameStart`。
> 也就是说，**真正发出开局请求的是对话框里的那个「確定」**，而且它比点击晚约 2 秒才回包。

> **第 6 步的匹配准备画面是这一轮实测才发现的，也推翻了旧说明的一个结论。**
> 上一版说「点完三个结算画面就回大厅」，这一轮把整条链路重走了一遍，结论是**回不到大厅**：
> 16:15:41.339 点道具面板的「確定」→ 屏幕变成「寶牌雙刃」规则说明 + 橙色
> 「匹配對局」按钮，**wire 上同样 0 帧**；
> 16:19:47.654 点那个橙色按钮 → **16:19:48.643 `.lq.Lobby.startUnifiedMatch`**
> → 16:19:53.433 `NotifyMatchGameStart` → 16:20:19.057 東1 开打（本次我坐 seat 2）。
> 这条路**不需要确认框**，比「再來一場」还直接，所以模块把它留作**兜底**：
> 万一「再來一場」不在了（另外三个人都走了），点掉三个结算画面后正好站在
> 这个画面上，点一下就能把客户端送进下一局。

坐标的来历（可复现）：窗口 2266×1383 物理像素下，「確定」的金色板 bbox
`(1414,1056)-(1630,1118)`，「再來一場」的蓝色板 bbox `(1170,1054)-(1384,1114)`，
对话框「確定」的金色板 bbox `(558,884)-(820,946)`，
「匹配對局」橙色板 bbox 约 `(1070,922)-(1207,990)`；
画布矩形由 AkagiMS 自己日志里的 `x_norm → css_x` 换算关系解出
（`css_x = 69.3134·x_norm + 0.4818`，`dpr = 1.5`），得 canvas 物理原点
`(4.4, 216.5)`、尺寸 `1663.5 × 936.3`，再折回 16:9 归一化。

> **2026-11-20 21:0x，在 v3 自己拉起的 Chromium 里把整条链路又走了一遍**
> （这次是用 CDP 直接派发鼠标事件 —— 和模块走的是同一条
> `Input.dispatchMouseEvent` 路径），四段全部实测通：
> 點「確定」→ 禮物盒 → 點「確定」→ 道具奖励面板 →
> 點「再來一場」→ 弹出「要再進行一場　寶牌雙刃・四人東　の匹配對局嗎?」→
> 點框裡的「確定」→ **21:05:34.684 `.lq.Lobby.startUnifiedMatch`**
> → **21:05:39.666 `NotifyMatchGameStart`** → 下一局開打（bot 正常出牌）。
>
> 這一輪還解出了**座標在 v3 視窗裡的落點**：v3 的畫布是 1036 × 582 CSS px、
> `y = 95`、內容 2560 × 1440（`dpr = 1.5`），於是
> 「確定」落在 **(944.7, 636.3)**、「再來一場」落在 **(791.9, 634.3)**、
> 對話框的「確定」落在 **(426.1, 528.9)** —— **三個都在按鈕本體內**，
> 沒有偏移。所以**座標不用重新校準**，模組換到 v3 自己的瀏覽器裡一樣能用。

> **还有第五个画面，但不用另外配坐标。** 半庄中途每打完一手（東1 打完了还要接着打東2），
> 会出一张**单局结束**画面，右下角是**蓝色**的「(N) 確認」—— 实测抓到的是「(2) 確認」，
> bbox `(1453,1069)-(1608,1124)`、中心 `(1530,1097)`、归一化 **(14.67, 8.46)**。
> 它和金色「確定」几乎同位，配置值 `(14.59, 8.37)` 同样落在它上面。
> 而且这张画面 **AkagiMS 自己就会点**（实测：15:36:41 画面出现 → 15:36:56 发出
> `confirmNewRound` → 東2 直接开打），本模块根本碰不到它。

### 4.3 wire 上到底有什么（以及**没有什么**）

**点掉结算画面，一个协议包都不发。** 实测：连点三下「確定」，uplink 上 **0 帧**
—— 没有请求，没有通知。AkagiMS 自己的自动点击走过同样的三个画面，同样是 0 帧。
**连「道具面板 → 匹配准备画面」那一步也是 0 帧。**
点「再來一場」也一样 —— 只弹出对话框，**0 帧**。

所以「等客户端回包、确认画面已清空」这条路**走不通**。本模块改用真正存在的三个信号：

| 信号 | 含义 | 模块的反应 |
|---|---|---|
| `.lq.Lobby.startUnifiedMatch` | 客户端请求开局 | **成功**，立即停止点击 |
| `.lq.Route.requestRouteChange` 且 `route_id = "route-3"` | 客户端**切到了匹配准备路由** = 走到了那张「寶牌雙刃」规则卡 + 橙色「匹配對局」按钮的画面 | **改点「匹配對局」**（直接开局），然后停手 |
| `.lq.Route.requestRouteChange` 且 `route_id = "route-4"` | 客户端**重载了大厅路由** | **停止**：大厅里没有「再來一場」，再点就落到大厅自己的按钮上了 —— 实测点「確定」的坐标会开出一局段位场，详见 §4.6.2 |

> 顺带纠正一个很容易误解的名字：`.lq.FastTest.confirmNewRound` **不是**「离开结算画面」的信号。
> 实测它出现在**开局**时 —— 在 `NotifyMatchGameStart` 之后 2 分 51 秒、在点「確定」后 23 毫秒，
> 而且第一局在同一毫秒开始。它的意思是「确认新一局（准备）」。
> （早期版本的续局模块就是靠它判断「结算已清空」，因此会一直空点直到预算耗尽 —— 已修。）

> **还有第二个被推翻的结论：`fetchAccountInfo` 不是「回到大厅」的信号。**
> 上一版说明把它当成了大厅信号，这一轮实测证明是错的：
> `.lq.Lobby.fetchAccountInfo` 在**一局结束后约 24 秒**就会发一次
> （15:45:31.896 `NotifyGameEndResult` → 15:45:56.778 `fetchAccountInfo`），
> 那时候**終局积分板还亮着** —— 因为大厅侧要刷新一次账号（段位变了）。
> 同一时刻**没有任何路由切换**，所以它压根没在说「客户端到了大厅」。
> 同样形状的消息在 13:35:08 也出现过一次（`logReport` 后 23.9 秒），同样不是回大厅。
> **换成路由切换才准确** —— 但下一条要说清楚它到底准不准。

> ⚠️ **这一轮把「路由切到 route-4 = 回到大厅」这个结论也修正了。**
> `route-4` 准确的意思是「**大厅路由被重载了**」，**不等于**「客户端正站在大厅画面上」。
> 实测抓到过两次「对不上」的 `route-4`：
> - 13:51:49.565 —— **对局进行中**（bot 正在打牌）客户端就重载了一次大厅路由；
> - 16:19:15.565 —— 客户端在**匹配准备画面上静置了 3 分 38 秒**之后，
>   自己重载了大厅路由（16:19:19.130 `requestRouteChange route-4`）。
>
> 也就是说 `route-4` 是客户端**自己按自己的节奏**做的事，不能当成「我已经回大厅了」来用。
> 好消息是：**结算流程本身根本不会走到大厅**（§4.2 第 6 步），
> 所以「確定」这个危险坐标**没有机会被点在大厅里** —— 这是实测出来的保证，
> 不是靠信号猜的。模块仍然保留 `route-4` 作为**兜底停手条件**
> （万一有别的东西把客户端送回了大厅），但它不是流程的一步。
>
> 顺带把路由编号也测清楚了（`.lq.Route.requestRouteChange` 的 `route_id`）：
> `route-2` = 段位場大厅、`route-3` = 匹配准备（等对手）、
> `route-4` = **大厅**、`route-5` = 对局中。

> ⚠️ **这一轮把「route-3 = 确认框已弹出」这个结论也推翻了 —— 而且它推翻的正是模块失效的原因。**
> 在 v3 自己的 Chromium 里把整条链路逐帧看下来：
> 点「再來一場」时，**协议里一个包都没有** —— 没有请求、没有通知、
> **也没有任何路由切换**；整条链路里 `route-3` **一次都没出现过**。
> 对话框（「要再進行一場…の匹配對局嗎?」）在 wire 上**完全隐形**。
> 而点框里的「確定」，客户端在**同一个毫秒**就发出了 `startUnifiedMatch`。
>
> 所以：**没有信号能告诉你对话框弹出来了**。
> 旧模块等 `route-3` 才去点对话框的「確定」，于是这个「確定」**一次都没被点过**；
> 接着它又去点「確定」—— 那一下落在了**对话框背后的道具面板**上，
> 把客户端从对话框底下走掉了。这就是它「按了 12 下却一场都没开」的全部原因。
>
> **现在的做法**：点完「再來一場」**立刻**点对话框的「確定」，不靠任何信号；
> 靠 `startUnifiedMatch` 判断成不成功。`route-3` 的含义也改回它本来的意思 ——
> 「客户端走到了匹配准备画面」，那时该点的是「匹配對局」。

### 4.4 点击顺序：为什么先点「再來一場」，以及对话框是「盲点」的

「再來一場」是**要下一局**的按钮，「確定」是**把当前画面点掉**的按钮 ——
两者都只在结算流程里存在。模块**每轮固定点三下**：

```
第1下 再來一場     → 終局积分板上没有，点了等于空点（落在自己的手牌墙上，实测无副作用）
                    → 弹不出对话框也没关系，下一下照样去点框里的位置
第2下 对话框確定   → 如果对话框在（只有道具奖励面板才有「再來一場」），这里命中！
                    → 客户端发出 startUnifiedMatch ⇒ 成功，停止
                    → 如果对话框不在，这一下落空，无害
第3下 確定        → 画面1 → 画面2（禮物盒）→ 画面3（道具奖励）
                    → 走完三屏后客户端站在「匹配准备画面」，下一轮改点「匹配對局」
```

实测走一遍（v3 的 Chromium，21:0x）就是第 1~3 下先空点两次，
到道具奖励面板上第 1 下命中「再來一場」、第 2 下命中对话框的「確定」，
**21:05:34.684 发出 `startUnifiedMatch`**。

这样排有三个好处：

1. **每一屏在被点掉之前，都先被「再來一場」点过一次。**
   不管结算流程是 1 屏、2 屏还是 3 屏，带「再來一場」的那一屏永远轮得到它 ——
   如果把「確定」排在前面，遇到只有一屏的流程就会先把那一屏点掉、再也点不到「再來一場」。
2. **对话框一弹出来就马上被回答，而且是「盲点」。**
   对话框只在「再來一場」之后存在，但它在协议里**没有任何消息**（§4.3），
   所以模块**不等信号**，点完「再來一場」直接就去点框里的「確定」。
   弹出来了就命中，没弹出来就落空 —— 两种情况都不会出事。
3. **「確定」排在后面** —— 它是唯一会「把当前画面点掉」的动作，
   排在后面才能保证「再來一場」总是先被尝试，而且对话框不会被它从背后顶掉。

这也正是雀魂客户端自己的行为：**「再來一場」是在回大厅之前出现的**，
根本不需要先回大厅、再从大厅里点房间条目 —— 模块也是这样做的。

**兜底**（实测第 6 步的匹配准备画面）：如果「再來一場」始终不出现
（另外三个人都走了的情况），模块会继续点「確定」，把三个结算画面点完，
正好站在**匹配准备画面**上 —— 这时点那个橙色「**匹配對局**」按钮（`(10.87, 7.10)`）
就会**直接开局**，同样发出 `startUnifiedMatch`，不需要确认框。
客户端切到匹配准备路由（`route-3`）时，模块就是靠这个信号知道该改点它的。

**预算是按「每轮 3 下」算的**：走完三屏要 3 轮 = 9 下，再用 1 轮点对话框 = 12 下；
兜底再多 1 轮点「匹配對局」= 15 下。所以 `max_confirm_presses` 默认给到 **36** ——
多留很多余量，因为**结算画面是带动画的，动画没走完时点下去完全不响应**
（实测：那种点击一个字节都没换），得再点一遍。

### 4.4.1 「確定」会被禮物盒的「獲得奖励」浮层吞掉很久（实测）

**2026-11-20 22:16 的一次失败**，日志原文（模块只用了 13 下就停了）：

```
22:16:11.727 requeue: game 1 ended — walking the result screens
22:16:13.742 requeue: pressing 再來一場 at (12.23, 8.34)
22:16:16.650 requeue: pressing 對話框的確定 at (6.58, 6.71)
22:16:18.683 requeue: pressing 確定 at (14.59, 8.37)      ← 終局积分板 → 禮物盒
22:16:21.588 requeue: pressing 再來一場 at (12.23, 8.34)
22:16:24.516 requeue: pressing 對話框的確定 at (6.58, 6.71)
22:16:26.567 requeue: pressing 確定 at (14.59, 8.37)      ← 落空
22:16:34.486 requeue: pressing 確定 at (14.59, 8.37)      ← 落空
22:16:42.367 requeue: pressing 確定 at (14.59, 8.37)      ← 落空
22:16:44.614 (wire) .lq.Route.requestRouteChange            ← route-3
22:16:44.732 requeue: pressing 匹配對局 at (10.87, 7.1)     ← 落空
22:16:45.600 requeue: the press budget ran out — the client was not put back into a game
```

事后抓屏才看清原因：**禮物盒这一步还多一层「獲得奖励」浮层**
（大标题 + 奖励道具格子），浮层在的时候**「確定」按钮根本没画出来**，
`(14.59, 8.37)` 落在浮层后面的角色立绘上 —— 点多少次都没用。
浮层退掉之后，**同一个坐标**再点一下就正常了（→ 道具奖励面板，
画面上「再來一場」「確認」两个按钮清清楚楚）。

而且 **`route-3` 也不等于「按钮已经画出来了」** —— 22:16:44.614 路由切到
匹配准备时，客户端还停在禮物盒浮层上，模块 118 ms 后就去点「匹配對局」，
那一下落空了。

所以补丁改成两件事：

1. **匹配對局 点完没有反应，不再直接放弃**，而是继续走下一轮
   （`再來一場` → 对话框確定 → 確定），最多连点 4 次「匹配對局」。
   **坐标没问题 —— 早停才是那次失败的全部原因**（36 下的预算只用了 13 下）。
2. **`route-3` 只当「可以试一下匹配對局」的提示**，不当「画面已经就绪」的证明。

> 结论：模块**不会**因为某一屏点不动就放弃整轮；
> 只要预算没用完，它会一直走下去。

**修复后的实测（2026-11-20 22:49~22:50，全程窗口被完全盖住）**：

```
22:49:xx  requeue: game 1 ended — walking the result screens
         requeue: pressing 再來一場 at (12.23, 8.34)       × 7 轮
         requeue: pressing 對話框的確定 at (6.58, 6.71)
         requeue: pressing 確定 at (14.59, 8.37)
22:50:14.638  (wire) .lq.Lobby.startUnifiedMatch
22:50:14.718  requeue: the match request went out after 20 press(es)
22:50:14.720  requeue: the client asked for a match — the next game is on its way
22:50:19.495  (wire) .lq.NotifyMatchGameStart          ← 新一局真的开打了
```

**共 20 下**（预算 36），全程那个 1400×900 的置顶遮挡窗一直盖着浏览器，
日志里 **`no input command` 警告 0 条**。

**紧接着的下一局（23:08~23:09，遮挡窗已撤掉、前台状态）又成功了一次**：

```
23:08:33.221  requeue: game 2 ended — walking the result screens
23:09:10.347  (wire) .lq.Lobby.startUnifiedMatch
23:09:10.425  requeue: the match request went out after 14 press(es)
23:09:10.426  requeue: the client asked for a match — the next game is on its way
23:09:16.077  (wire) .lq.NotifyMatchGameStart
```

**两次用的下数不一样（20 下 / 14 下）** —— 因为「確定」会不会被「獲得奖励」浮层
吞掉是随机的，吞掉时模块就多走几轮。**这正是「点空不放弃」这条修复的价值**：
下数会浮动，但两次都接上了。

> ⚠️ **旧版本为什么一场都没开出来**：它等 `route-3` 才去点对话框的「確定」，
> 而 `route-3` 在这条链路上**从不出现**（§4.3）—— 对话框的「確定」一次都没被点过；
> 接着它去点「確定」，那一下又落在对话框背后的道具面板上，把客户端从对话框底下走掉了。
> 现在改成「点完『再來一場』立刻点对话框的『確定』」，不再依赖任何信号。

### 4.5 前端设置

打开 **设置 → 自动打牌**，拉到最下面就是「一局结束后自动开始下一局」区块：

| 界面项 | 对应配置 | 默认值 | 说明 |
|---|---|---|---|
| 一局结束后自动开始下一局 | `enabled` | 关 | 总开关 |
| 局数上限 | `max_games` | **0（不限）** | 0 = 不限。**改完立即生效**，下一局结束就用新额度；**改数字不会把计数归零**（见 §4.11） |
| 停止时间 | `stop_at` | **空（不限）** | 每天的截止时刻 `HH:MM`，过了这个点就不再开新局（见 §4.11） |
| 「已打 N · 还剩 M」+「重置计数」 | — | — | 后端实时计数；「重置计数」把计数归零，重新开一批 |
| 自动点击「再來一場」按钮 | `queue_button` 开关 | **开** | 关掉就只点掉结算画面 |
| 「再來一場」X / Y 坐标 | `queue_button` | `(12.23, 8.34)` | 实测值，一般不用改 |
| 结算画面「確定」按钮坐标 | `result_confirm` | `(14.59, 8.37)` | 实测值，三个画面共用 |
| 「再來一場」对话框里的「確定」坐标 | `queue_dialog_confirm` | `(6.58, 6.71)` | 实测值，**真正发出开局请求的就是它** |
| 自动点击「匹配對局」按钮 | `match_button` 开关 | **开** | **兜底**：走到匹配准备画面时直接开局 |
| 「匹配對局」X / Y 坐标 | `match_button` | `(10.87, 7.10)` | 实测值，一般不用改 |
| 最多点击次数 | `max_confirm_presses` | **36** | 整个流程的总预算，兜底上限。每轮点 3 下，走完三屏 + 点对话框要 12 下；**实测成功那一轮用掉了 17 下**，所以余量给得比较宽 |
| 最多「確定」点击次数 | `max_dismiss_presses` | **15** | 结算画面就是 3 个，但**动画期间点下去会白扣一次**（实测 5 次「確定」里有 3 次被动画吞掉），所以留了余量。**填 0 = 不单独限制** |
| 额外按钮坐标（活动 / 道具弹窗） | `extra_buttons` | 空 | 见 §4.6 |
| 点击节奏（高级） | 下面 4 项 | 见下 | 首次点击延迟 / 每步点击后等待 / 点击间隔 / 点击后等待回包 |

> **「局」怎么算**：触发点是服务端的 `NotifyGameEndResult`，也就是**一局半庄结束**。
> 所以 `max_games = 10` 是 **10 局半庄（约 40~50 手）**，不是 10 手。
> AkagiMS 的历史记录也是同一个口径（它的 6 条记录，每条 `round: 5`，即一局 5 手）。

默认值：

```
result_delay_ms          = 2000   # 一局结束后等多久点第一下
step_wait_ms             = 2000   # 每点一下之后等多久（结算动画的落定时间）
between_press_delay_ms   = 0      # 每点一下之后额外等多久（真正的间隔由上面那项决定）
queue_confirm_timeout_ms = 1200   # 点完对话框「確定」后等多久看 startUnifiedMatch
max_confirm_presses      = 36     # 整个流程最多点几下（每轮 3 下）
max_dismiss_presses      = 15     # 其中最多几下落在「確定」上（0 = 不限）
hover_delay_ms           = 400    # 与 AkagiMS 默认一致
click_hold_ms            = 300
```

同样的键在 `config.toml` 里是：

```toml
[autoplay]
enabled = true

[autoplay.requeue]
enabled = true
max_games = 0        # 0 = 不限
stop_at = ""         # 每天的截止时刻 HH:MM；空 = 不限
result_confirm = [14.59, 8.37]
queue_button = [12.23, 8.34]
queue_dialog_confirm = [6.58, 6.71]
match_button = [10.87, 7.10]
# extra_buttons = [[x, y], ...]   # 「活動」这类自带关闭按钮的面板才需要
max_confirm_presses = 36
max_dismiss_presses = 15
result_delay_ms = 2000
step_wait_ms = 2000
between_press_delay_ms = 0
queue_confirm_timeout_ms = 1200
hover_delay_ms = 400
click_hold_ms = 300
```

所有字段都可省略，省略即用默认值。

> 早期版本里的 `queue_delay_ms`（排队点击延迟）和 `max_queue_presses`（最多排队点击次数）
> 已经删掉了 —— 现在是一个循环，没有单独的「排队阶段」。旧配置文件里残留这两个键会被忽略。

### 4.6 「活動」「獲得道具」这类中途弹窗

实测结论：**「禮物盒」和「道具奖励」这两个面板的「確定」就在主按钮 `(14.59, 8.37)` 上**，
主按钮已经能关掉它们，不用额外配坐标。

只有「活動」通知这种**自带关闭按钮**的面板才需要单独填：把它的关闭按钮坐标填进
「额外按钮坐标（活动 / 道具弹窗）」，模块每轮会试着点一下它们
（顺序是 再來一場 → 对话框確定 → 额外… → 確定 → 匹配對局），收到客户端回包就停 ——
猜错最多浪费一次点击，不会点错东西。

> ⚠️ **不要把「再來一場」对话框里的「確定」当成「活動」弹窗去关。**
> 那个坐标 `(6.58, 6.71)` 点下去是**开局**，不是关窗口。

#### 4.6.1 大厅坐标地图（全部实机点开、点关过一遍）

在**繁体**客户端的大厅上，用系统级真实鼠标事件把每个面板都点开再点关，逐个量过。
下面全部是 16:9 归一化坐标（对游戏 canvas），窗口 2266×1383 下测得。

**顶栏四个挂牌**（y ≈ 0.80）

| 挂牌 | 打开 | 面板怎么关 | 关闭坐标 |
|---|---|---|---|
| 活動（猫形牌） | (11.00, 0.79) | 面板右上角米色圆角方块「‹」 | **(15.08, 2.12)** |
| 郵件（灯笼） | (12.32, 0.80) | 面板右上角返回箭头 | (14.71, 1.68) |
| 排名（灯笼） | (13.38, 0.80) | 面板右上角**灰色 X** | **(14.19, 1.18)** |
| 公告（灯笼） | (14.45, 0.80) | 面板右上角**红色菱形 X** | (13.60, 1.68) |

**中部三张吊牌**：段位場 (11.09, 2.80)、比賽場 (11.09, 4.51)、友人場 (11.09, 6.37)

**底部导航**（整排占 y ≈ 8.1~8.8，点图标或文字都行）

| 宿舍 | 友誼 | 觀戰 | 牌譜 | 倉庫 | 商店 | 募集 |
|---|---|---|---|---|---|---|
| 8.00 | 9.19 | 10.42 | 11.57 | 12.68 | **13.71** | **14.98** |

**段位場 列表页**（銅之闇 / 銀之闇 / 金之闇 / 王之闇）：左上角「‹ 返回」箭头 **(9.91, 2.29)**。
> ⚠️ 列表里的**房间条目点了就会开真对局** —— 别点。

> **坐标一定要用像素法测，不要用目测。** 排名面板那个灰色 X，目测偏了 32 个物理像素，
> 真实值是 (14.19, 1.18)。用「找浅色控件」脚本框出 bbox 再折算归一化坐标才靠谱。

#### 4.6.2 ⚠️ 在大厅里点这三个坐标会出什么事（实测）

续局模块一旦进了大厅还继续点，就会落到大厅自己的 UI 上。实测结果：

| 在大厅点 | 命中 | 实测后果 |
|---|---|---|
| 再來一場 `(12.23, 8.34)` | 牌譜与倉庫之间的**空隙** | **完全没反应**（画面不变、wire 无变化）—— 安全 |
| 对话框確定 `(6.58, 6.71)` | 角色头发 | **完全没反应** —— 安全 |
| 確定 `(14.59, 8.37)` | 大厅右下方 | **危险**：会把客户端送进**段位場**并开一局，见下 |

点「確定」那个坐标后的实测时间线（`route_id` 是关键）：

```
15:30:23.771  .lq.Route.requestConnection     route-2      ← 点击后 0.8 秒
15:30:27.116  .lq.Route.requestRouteChange     route-4 → route-2
15:30:27.184  .lq.Lobby.prepareLogin
              （画面从大厅切到「牌楼 + 彩虹」场景，带加载转圈）
15:31:02.940  .lq.Lobby.startUnifiedMatch  {"match_sid":"2:55"}   ← 开了一局段位场
15:31:06.706  .lq.NotifyMatchGameStart
```

`route-4` = 大厅、`route-2` = 段位場大厅 —— 所以这一下点**确实**把客户端从大厅
送进了段位場，段位場那边随即自动排了一局（`match_sid` 的 `2` 就是段位場）。
整个过程 **AkagiMS 一个动作都没有**（bot 是「已停止」状态），那个开局请求是
**客户端自己**发出去的。

对照实验：**客户端不会自己排队。** 同样在大厅里，15:28:03 → 15:30:23 整整 90 秒，
wire 上只有心跳，**一次 `startUnifiedMatch` 都没有**。所以「排队」一定是被点击触发的。

这就是续局模块必须**停手**的原因。
这一轮把整条结算链路重走了一遍，还发现了一个**更有力的保证**：
**结算流程根本不会走到大厅** —— 道具面板的「確定」通向的是**匹配准备画面**
（§4.2 第 6 步），不是大厅。所以「確定」这个危险坐标**没有机会被点在大厅里**，
这是实测出来的，不是靠信号猜的。
两层保险仍然是：模块盯着**路由切到 route-4** 这个信号停手（虽然它更多是兜底，
因为客户端会自己重载大厅路由，见 §4.3），加上 `max_dismiss_presses`
（默认 3 = 结算画面的个数）这个上限。

（另外两个实测细节：**排名面板的 ESC 关不掉**，必须点坐标；
**段位場 列表滚不动**，四个段位就是全部。）

### 4.7 窗口拉小还能用吗？需要图像识别吗？

**不需要图像识别。** 坐标是相对**游戏 canvas** 的 16:9 归一化坐标，而 canvas 本身
会随窗口等比缩放，UI 在画面里的相对位置永远不变。实测（真实 Chrome 加载
`https://game.maj-soul.com/1/`）：

| 窗口 | canvas CSS 尺寸 | 比值 |
|---|---|---|
| 1600×1000 | 1600×900 | 1.000 |
| 1120×700 | 1120×630 | **0.700** |

宽高比恒为 16:9，**绘制缓冲恒为 2560×1440**（不随窗口变）—— 整幅画面等比缩放而已。
代码上也一样：`CanvasRect::pixel()` 用的是**每次点击时实时读到的**
`getBoundingClientRect()`，不是启动时缓存的尺寸。

> **一个必须分清的区别**：以上说的是 **v3 自己开的浏览器**。
> AkagiMS 用的是它自己的 WebView2，那个 canvas **不缩放、只裁切** ——
> 实测窗口缩到 1538×918 时，「確認」直接被裁出画面外。
> 这是 AkagiMS 自己的实现问题，不是雀魂的问题；v3 里不存在。

### 4.8 日志

模块的每一步都会打日志：

```
requeue: game 1 ended — walking the result screens
requeue: pressing 再來一場 at (12.23, 8.34) — page (781.2, 583.9)
requeue: pressing 對話框的確定 at (6.58, 6.71) — page (420.3, 479.9)
requeue: pressing 確定 at (14.59, 8.37) — page (931.9, 585.8)
...
requeue: the match request went out after 20 press(es)
requeue: the client asked for a match — the next game is on its way
requeue: the client went back to the lobby — no match was asked for (the 再來一場 control is only there while the others stay)
requeue: the press budget ran out — the client was not put back into a game
requeue: reached the configured maximum of 10 game(s) — stopping
requeue: the batch limit changed to 20 — the counter starts over
```

> 上面那 20 下是**实测成功那一轮的真实数字**（2026-11-20 22:50:14，
> 窗口全程被完全盖住）；更早一轮 21:55:41 在前台用掉 17 下成功。
> **旧版模块的 12 下预算在同一位置三局半庄全部失败。**

### 4.9 万一「再來一場」的位置变了，怎么自己校准

`result_confirm`、`queue_button`、`queue_dialog_confirm`、`match_button` 都是实测值，正常不用动。
真要校准的话：

1. 让雀魂停在**道具奖励面板**（第 3 步那个有蓝色「再來一場」的画面）。
2. 截一张图，找到「再來一場」按钮中心，记下它相对于**游戏 canvas**的像素位置。
3. 换算成 16:9 归一化坐标：`x_norm = (px - canvas_left) / canvas_w * 16`，
   `y_norm = (py - canvas_top) / canvas_h * 9`。
4. 填进设置界面的「再來一場 X / Y 坐标」。
5. 点一下「再來一場」，等那个对话框弹出来，量它的金色「確定」中心，
   同样换算成归一化坐标，填进「「再來一場」对话框里的「確定」坐标」。
6. 「匹配對局」按钮：在道具奖励面板点「確定」，等画面变成「寶牌雙刃」规则说明，
   量那个橙色大按钮的中心，填进「「匹配對局」X / Y 坐标」。
   （这一步不要点那个按钮 —— 一点就开局了。量好坐标按 ESC 或切走即可。）

> canvas 矩形（CSS 像素）可以用同一套办法量：`page.evaluate` 取
> `document.getElementById('unity-canvas').getBoundingClientRect()`。

**校准技巧**：把「最多点击次数」临时设成 1，然后看日志里的
`requeue: pressing 確定 at (14.59, 8.37) — page (944.7, 636.3)`，
用 `page_x / (canvas_w/16)` 反推你当前画面下任何点的归一化坐标。

校准好之后，模块每次点对话框里的「確定」都会等 `startUnifiedMatch` 回来确认；
没回来就继续按预算走，所以坐标没对准也不会乱排队。

### 4.10 风险

- 段位场连挂的**封号风险由你自己承担**，工具不提供任何规避手段。
- **模块一旦进了大厅就绝不能再点。** 实测在大厅里点「確定」的坐标
  `(14.59, 8.37)` 会把客户端送进**段位場路由**并**开出一局段位场**（§4.6.2）。
  三层保险：(1) 实测**结算流程根本不会走到大厅**，那个坐标没有机会落在大厅里；
  (2) 模块盯着**路由切到 route-4** 这个信号停手（注意它只是兜底，
  客户端会自己重载大厅路由，见 §4.3）；
  (3) `max_dismiss_presses` 默认 8（结算画面只有 3 个，多出来的余量是给动画的）。
  `queue_confirm_timeout_ms` 默认 **1200 ms** —— 实测点对话框的「確定」后，
  客户端在**同一个毫秒**就发出了 `startUnifiedMatch`（`21:05:34.684`），
  所以 1200 就够；**别把它调到 200 以下**，那样反而会错过成功信号。
- 坐标是手标的：**雀魂客户端一改 UI，按钮就会挪位**，届时需要重新校准。
  模块在点不到时会在日志里报 `dispatch_click failed` 或 `lands outside the canvas rect`，
  不会静默乱点。
- 「再來一場」是**再战**（同一桌的另外三个人还没走）。别人都走了就没有这个按钮 ——
  这时模块会走**兜底**：点完三个结算画面停在匹配准备画面，点「匹配對局」直接开局。
  这一条**还没有在「另外三个人都走了」的真实场景下跑过**，属于未验证项。

---

### 4.11 运行时可调的打牌计划（剩余局数 / 截止时间 / 「这局后停」）

悬浮窗（一直贴在屏幕上的那个小窗）在建议列表下面多了一行：

```
已打 12 · 剩 8      [这局后停]   [23:30]
```

| 控件 | 作用 | 生效时机 |
|---|---|---|
| 左边的计数 | 已经打完几局、还剩几局（额度不限时显示「不限」） | 每 2 秒刷新 |
| 「这局后停」 | 一键把局数额度设成「当前这一局的序号」—— 这一局打完就不再开下一局 | 下一局结束、决定要不要续局的那一刻 |
| 时间框 | 每天的截止时刻（`HH:MM` 本地时间），过了这个点就不再开新局 | 同上 |

**不用重启 v3，也不用等下一局开始** —— 三个控件都直接写进 `config.toml`，
而续局循环判断「要不要续下一局」的那一刻读的正是最新的配置。

#### 局数上限的语义变了：改数字不再归零

老版本里 `max_games` 是「一批」的额度：你在设置里把它从 20 改成 5，计数会**清零**，
于是它会**再打 5 局**。这跟「还剩几局」没法同时成立 —— 你没法在打到第 12 局的时候
说「再打 8 局就停」。

现在 `max_games` 是**整轮的额度**：

| 操作 | 结果 |
|---|---|
| 设成 N | 总共打到第 N 局就停（**改数字不会把计数归零**） |
| 调小 | 提前停 |
| 调大 | 继续打 |
| 想从头再来 | 设置页点「重置计数」，或者直接删掉 `requeue_progress.json` |

「这局后停」算的就是「当前计数 + 1」，所以按钮给出的数永远跟这一局对得上。

#### 计数跨重启保留

计数写在配置文件旁边的 **`requeue_progress.json`** 里（和 `config.toml` 同目录）。
v3 崩了或者你手动重启之后，额度接着算 —— 老版本重启就归零了。

#### 截止时间怎么理解

`stop_at` 是**一天里的某个时刻**，不是「还剩 N 分钟」：

- 凌晨 1 点设成 `03:00` → 意思是「打到三点」；
- 上午 10 点设成 `03:00` → 意思是「三点以后不再开新局」。

判断发生在**两局之间**：正在打的那一局一定会打完，只是不再开下一局。
过了零点按当天的时刻重新算（所以 `23:30` 在凌晨 0:10 不触发 —— 那是「每天 23:30 之后不再开新局」）。

填的时间读不出来（不是 `HH:MM`）会被**当场拒绝**，并在日志里记一条 warn ——
存一个永远不触发的值比报错更糟。

#### 配置文件

```toml
[autoplay.requeue]
max_games = 20
stop_at = ""       # 每天的截止时刻，HH:MM 本地时间；空 = 不限
```

> `stop_at` 是**字符串**而不是「可空」字段：配置文件是**合并**写回磁盘上已有的内容的，
> 而空值在合并时根本不写 —— 那样的话一旦设过就再也清不掉（下次启动又回来了）。
> 空串才是「没设」。

#### 日志

```
requeue: the plan was changed — max_games = 8, stop_at = "23:30"
requeue: stopping after the game in play (budget set to 13)
requeue: the counter was reset
requeue: stopping — the configured maximum of 8 game(s)
requeue: stopping — the configured cutoff 23:30
requeue: restored the counter at 12 game(s) from the last run
```

#### 四个新命令（给脚本用）

| 命令 | 作用 |
|---|---|
| `get_requeue_status` | 读当前计划：已打几局、还剩几局、截止时间 |
| `set_requeue_plan` | 改计划（`{max_games, stop_at, clear_stop_at}`，每个字段可选） |
| `stop_after_current_game` | 「这局后停」 |
| `reset_requeue_counter` | 计数归零 |

---

## 5. 后台运行（浏览器不必放在前台）

### 5.1 先拆两个问题：它到底是怎么点的，为什么非要前台

**问题一：它是 OS 级的假鼠标吗？**

不是。v3 和 AkagiMS **都用 CDP（Chrome DevTools Protocol）派发
`Input.dispatchMouseEvent`** —— 事件直接注入页面，**完全不经过操作系统的鼠标栈**。

对 AkagiMS 的 exe 做字节扫描，命中的是它自己的源码路径：

```
Input.dispatchMouseEvent      ← 紧挨着它自己的 src\autoplay\cdp.rs
Runtime.evaluate
Browser.getVersion
autoplay: CDP press failed
```

`SendInput` / `SetCursorPos` / `SetForegroundWindow` 这些**确实**在 AkagiMS 的 exe 里，
但它们**不是**它的点击实现 —— 它们和 `ClipCursor` / `ShowCursor` /
`MonitorFromPoint` 一起，出现在 **Tauri / WebView2 框架自己的 API 名单**里
（框标题、窗口置顶、坐标换算），跟打牌无关。

所以「模拟点击」这个说法要拆成两半看：

| | 操作系统层面 | 页面层面 |
|---|---|---|
| Akagi v3 + 本包 | **完全不碰**，你的鼠标指针照常能用 | CDP 合成 `mousedown/mouseup` 投到 canvas 上 |
| AkagiMS 全自動 | 同上 | 同上 |

**问题二：那为什么非要前台？**

因为**卡点不在点击，在渲染循环**。

雀魂客户端是 **Laya / Unity WebGL**。它的按钮命中**不是**靠事件监听器，
而是放在 **`requestAnimationFrame` 循环里轮询**当前指针状态：

```
rAF 停了 → 没人去轮询指针 → 点击事件「送到了」但**没人读**
```

Chrome 一旦认为窗口不用渲染（被完全遮挡 / 最小化），**rAF 就停**。
CDP 派发的点击**照样发得出去**，游戏那边**毫无反应**。

日志里的铁证（原文）：

```
autoplay: no input command after clicking Dahai { actor: 1, pai: "S", tsumogiri: false } at [...]
```

意思是「点击已经派发出去了，但游戏没有回任何输入指令」。
这些警告**只出现在窗口不可见的那段时间**；窗口一恢复，bot 立刻接着出牌。

### 5.2 实测帧率（改之前 / 改之后）

用 `Browser.setWindowBounds` 控制窗口，页内挂一个 rAF 计数器读真实帧率：

| 场景 | 改之前 | 改之后 | 能点吗 |
|---|---|---|---|
| 前台聚焦 | 60.0 fps | 60.0 fps | ✅ |
| **切到别的程序（失焦但完整可见）** | 60.0 fps | 60.0 fps | ✅ **本来就完全正常** |
| **被别的窗口完全盖住** | **0.0 fps** | **60.0 fps** | ✅ |
| **把窗口拖到屏幕外** | **0.0 fps** | **60.0 fps** | ✅ |
| **最小化** | 0.0 fps | **0.0 fps** | ❌ **无解** |
| 恢复 | 60.0 fps | 60.0 fps | ✅ |

**最小化无解。** 窗口最小化时 Windows 根本不给它绘制面积，Chrome 直接把渲染
挂起 —— 这是浏览器的行为，没有任何启动参数能改。
**只要别最小化就行**：被别的窗口盖住、拖到屏幕外、切到别的程序，
现在**全部不影响打牌**。

（AkagiMS 同理：它内嵌的 WebView2 在最小化时一样会停，这是同一套 WebGL 的脾气。）

### 5.3 怎么配

**设置 → 抓包模式 → Chromium → 「额外启动参数（extra_args）」**，填这三行：

```
--disable-backgrounding-occluded-windows
--disable-renderer-backgrounding
--disable-features=CalculateNativeWinOcclusion
```

| 参数 | 作用 |
|---|---|
| `--disable-backgrounding-occluded-windows` | 窗口被遮挡时不要进入「后台」状态 |
| `--disable-renderer-backgrounding` | 不要降低渲染器的优先级 |
| `--disable-features=CalculateNativeWinOcclusion` | 关掉 Windows 原生的遮挡检测（**关键的一条**） |

配好之后**重启 v3**，浏览器窗口就可以随便藏起来了。

### 5.4 v3 这里有个真 bug：不修的话最后一条参数是无效的

**Chrome 只认命令行上最后一个 `--disable-features`。**

v3 原来的代码把自己的
`--disable-features=TranslateUI,InterestFeedContentSuggestions`
写在 `extra_args` **之前**，于是你在 `extra_args` 里写的任何 `--disable-features`
都会把内置的那条**顶掉**（内置的 `TranslateUI` 会失效）。

补丁改成了：**把内置列表和你的列表合并成一条，并且排在 `extra_args` 之后**。
（`src/capture/chromium/launch.rs` 的 `merged_disable_features()`，
兼容 `--disable-features=X` 和 `--disable-features X` 两种写法，带 4 个单测。）

实机验证 —— 新版拉起的 Chrome 真实命令行：

```
--disable-backgrounding-occluded-windows --disable-renderer-backgrounding
--disable-features=TranslateUI,InterestFeedContentSuggestions,CalculateNativeWinOcclusion
https://game.maj-soul.com/1/
```

**只有一条 `--disable-features`，三个 feature 全在里面。**

### 5.5 实测：完全被盖住的时候 bot 照样在打

用一个 1400×900 的置顶窗口把雀魂**整个盖住**，之后 75 秒里：

- 日志里的弃牌数 **22 → 41**（游戏在正常推进）；
- **没有一条** `no input command after clicking`（改之前这段时间全是警告）；
- bot 不但出牌，连**碰（Pon）**和**大明杠（Daiminkan）**这种多步复合操作都点成功了。

也就是说：**窗口被盖住之后，v3 的自动打牌完全不受影响。**
配合 §5.2 的帧率表，结论就是：

> **别最小化，其余随便。** 窗口可以丢到屏幕外、可以压在别的软件底下、
> 可以切到别的程序后面 —— bot 照打不误。

---

## 6. 改动清单（31 个文件）

补丁文件：**`Akagi-v3增强补丁.patch`**（31 个文件，`git apply` 可直接应用；
已用 `git apply` + 逐文件字节比对验证过：应用后与工作树 **31/31 完全一致**）。

| 文件 | 改动 |
|---|---|
| `src/autoplay/requeue.rs` | **新增**：续局循环本体（每轮固定点三下：「再來一場」→ **立刻**点对话框里的「確定」（盲点，wire 上没有信号）→「確定」往前走一屏；走到匹配准备画面（`route-3`）就改点「匹配對局」兜底） |
| `src/autoplay/lobby_watch.rs` | **新增**：客户端计数器（`startUnifiedMatch` / `match_prepared`(route-3) / `lobby_entered`(route-4) / `confirmNewRound`） |
| `src/autoplay/context.rs` | `AutoplayContext` 增加 `lobby_watch` 槽位 |
| `src/autoplay/manager.rs` | `EndGame` 钩子 + 局数计数 + 配置变更时重新计数 |
| `src/autoplay/mod.rs` | 导出 `lobby_watch` 模块 |
| **`src/autoplay/delay/mod.rs`** | **`RandomDelayConfig::extra_for()`：随机延迟采样 + 碰/吃除外** |
| **`src/autoplay/majsoul/mod.rs`** | **`push_pre_delay()` 里叠加随机延迟，并按服务器预算卡住上限** |
| `src/bridge/mod.rs` | `BridgeHooks` 增加 `lobby_watch` |
| `src/bridge/majsoul/mod.rs` | bridge 识别大厅请求并计数（开局请求 + 路由切换 route-3/route-4） |
| `src/capture/chromium/mod.rs` | 把 `lobby_watch` 接进 bridge |
| **`src/capture/chromium/launch.rs`** | **`merged_disable_features()`：把内置和用户的两份 `--disable-features` 合并成一条并排在最后**（Chrome 只认最后一个）—— 见 §5.4 |
| `src/config/autoplay.rs` | 新增 `RequeueConfig` + **`RandomDelayConfig`** + `AutoplayConfig.requeue` / `MajsoulAutoplayConfig.random_delay` 字段 |
| `src/config/mod.rs` | 导出 `RequeueConfig` + **`RandomDelayConfig`** |
| `tauri.conf.json` | **identifier 改成 `com.akagi.akagi.v3`**（见 §7） |
| `frontend/src/types.ts` | `RequeueConfig` 类型 + **`RandomDelayConfig` 类型** |
| `frontend/src/routes/Settings.tsx` | 设置面板的续局区块 + **随机延迟区块** |
| **`frontend/src/components/NativeApiFields.tsx`** | **模型下拉选择（`ModelPicker`）+ 打开界面自动拉模型列表** |
| `frontend/src/i18n/resources/*.json` | 四语文案（新增 11 键 × 4 语） |
| **`src/autoplay/plan.rs`** | **新增**：打牌计划 —— 计数器、`HH:MM` 截止时刻的解析与判定、计数的落盘 |
| **`src/config/merge.rs`** | **回归测试**：空值必须能覆盖文件里的旧值（否则「清除截止时间」清不掉） |
| **`src/ipc/commands.rs`** | **新增四个命令**：`get_requeue_status` / `set_requeue_plan` / `stop_after_current_game` / `reset_requeue_counter` |
| **`src/ipc/state.rs`** | `AppState` 增加 `requeue_plan`（后端与前端共享的计数器） |
| **`src/ipc/overlay.rs`** | 悬浮窗高度给计划行留出 26px（`PLAN_BAR_HEIGHT`） |
| **`src/lib.rs`** | 把共享的计数器交给 autoplay manager |
| **`frontend/src/routes/Overlay.tsx`** | 悬浮窗底部挂上计划行 |
| **`frontend/src/components/RequeuePlanBar.tsx`** | **新增**：计划行组件（计数 + 「这局后停」+ 截止时间） |
| **`frontend/src/components/RequeuePlanBar.test.tsx`** | **新增**：计划行的前端单测 |
| **`frontend/src/lib/requeuePlan.ts`** | **新增**：计划的轮询 hook |

---

## 7. 为什么改了 identifier（重要）

上游 v3 的 Tauri identifier 是 `com.akagi.akagi`，**和 AkagiMS 一样**。
WebView2 的浏览器进程是按**用户数据目录**（`%LOCALAPPDATA%\<identifier>\EBWebView`）
共享的，两个应用 identifier 相同就会共用同一个浏览器进程；而 MS 文档写明：

> As a browser process may be shared among WebViews, WebView creation fails with
> `HRESULT_FROM_WIN32(ERROR_INVALID_STATE)` if the specified options does not
> match the options of the WebViews that are currently running in the shared
> browser process.

实测就是这个：AkagiMS 开着时启动本构建，日志里两个 webview 全部
`failed to create webview: HRESULT(0x8007139F)`，**窗口根本出不来**；
关掉 AkagiMS 再启动，立刻正常。

所以补丁把 identifier 改成 `com.akagi.akagi.v3`，让两个应用各用各的
WebView2 数据目录。**改完之后实测：AkagiMS 和本构建同时运行，两个窗口都正常。**

---

## 8. 编译（Windows）

v3 的 Rust 依赖 `aws-lc-sys`、`candle` 等 C 依赖，Windows 上要一套完整的
C 工具链。这里踩过三个坑：

**① 链接器**：Rust 的 `x86_64-pc-windows-gnu` 目标会链接 `-lgcc -lgcc_eh
-lmingwex -lmingw32` —— 那是 **GCC 运行时**的库；LLVM-MinGW 只有 `libunwind`，
没有 GCC 运行时，直接链接会失败（`lld: error: unable to find library -lgcc_eh`）。
解法是用 **`x86_64-pc-windows-gnullvm`** —— Rust 官方把它做成了**一等 host**
（`rust-1.100.0-x86_64-pc-windows-gnullvm`），它的 std 链接的正是
`libunwind` + MSVC CRT，和 LLVM-MinGW 完全对得上。

**② tauri-build 2.5.6**：它按固定层级从 `OUT_DIR` 反推 cargo 的 target 目录，
而 cargo 后来把 `OUT_DIR` 从 `<pkg>-<hash>/out` 改成了 `<pkg>/<hash>/out`，
于是它算错一层，Windows-GNU 分支去读一个不存在的目录，build script 以
`系统找不到指定的路径 (os error 3)` 失败。上游在 tauri-build 2.7.0 修了，
但 2.7.x 要求 tauri-utils ~2.10，而本项目在 2.5 线上（capability/permission
schema 不通用），所以把上游那几行改法打到本地 registry 里的 2.5.6 副本上
（`akagi-src/patch_tauri_build.py`，`build.sh` 会自动执行）。
注意 cargo 不跟踪 registry 源的 mtime，所以打完补丁要把旧的产物删掉。

**③ crates.io 索引**：直连只有 ~3MB/分，700 个 crate 要两小时。
`toolchain/cargo/config.toml` 里配了
`sparse+https://mirrors.ustc.edu.cn/crates.io-index/`，实测满速。

本仓库里已经装好了整套工具链（都在 workspace 内，不动系统）：

```
workspace/toolchain/
  rustup/                 rustc/cargo 1.100.0，gnu + gnullvm 两个 host
  cargo/                  CARGO_HOME（crates.io 走 USTC 镜像）
  llvm-mingw/             LLVM-MinGW 20261117 msvcrt
  cmake/ nasm/ protoc/    aws-lc-sys 的构建依赖
```

一键编译：

```bash
cd workspace/akagi-src
bash build.sh        # 编译
bash check.sh        # 只做类型检查
bash test.sh         # 跑 Rust 单测
```

> `test.sh` 会把 Windows 商店的 `python3` 占位符（`…/Microsoft/WindowsApps/python3`）
> 从 PATH 里剔掉 —— 那个 stub 一执行就退 49 号码，会让两个「拉起 Python bot」的测试假失败。

产物：`Akagi-3/target/x86_64-pc-windows-gnullvm/release/akagi.exe`，
外加 `WebView2Loader.dll`（**必须和 exe 放同一个目录**，这是 WebView2 的动态加载器）。

打包好的可直接运行的版本在 `workspace/dist-akagi/`：

```
dist-akagi/
  akagi.exe             54,168,576 B
  WebView2Loader.dll       160,320 B
```

双击即可运行（首次运行会在 exe 旁边生成 `configs/`、`logs/`、`history/`、`ca/`）。

---

## 9. 已验证 / 未验证

### 已验证

- `cargo build --release --features custom-protocol --target x86_64-pc-windows-gnullvm` 成功，产出 exe **54,168,576 B**。
- **续局模块实战成功（2026-11-20 21:55）**：新版模块在一局结束后自己走完
  「再來一場」→ 对话框「確定」→ 確認，**共 17 下**，
  然后 **21:55:41.415 `.lq.Lobby.startUnifiedMatch`** → **21:55:41.457** 日志确认
  `requeue: the client asked for a match — the next game is on its way`。
  **旧版模块的 12 下预算在同一位置三局半庄全部失败**（日志原文：
  `requeue: the press budget ran out — the client was not put back into a game`）。
- **【最终验证】连续两局自动续局都成功（2026-11-20 22:49~23:09）**：
  - 第 1 局（**窗口被完全盖住**）：22:49:21 结束 → 走完三个结算画面 →
    **22:50:14.638 `startUnifiedMatch`** → 22:50:14.718 `the match request went out after 20 press(es)`
    → **22:50:19.495 `NotifyMatchGameStart`**，新一局开打。
  - 第 2 局（**前台**，遮挡窗已撤）：23:08:33 结束 → **23:09:10.347 `startUnifiedMatch`**
    → `the match request went out after 14 press(es)` → **23:09:16.077 `NotifyMatchGameStart`**。
  - 两次加起来 **34 下 / 预算 36 × 2**，会话内 **`no input command` 警告 0 条**。
- **后台状态下 requeue 自动续局成功（2026-11-20 22:49~22:50）**：
  窗口被一个 1400×900 的置顶窗口**整个盖住**的情况下，一局结束后模块自己走完
  三个结算画面 → 点「再來一場」→ 点对话框的「確定」，
  **22:50:14.638 `.lq.Lobby.startUnifiedMatch`** →
  **22:50:14.718 `the match request went out after 20 press(es)`** →
  **22:50:19.495 `NotifyMatchGameStart`**，新一局正常开打、bot 继续出牌。
  整个会话 **`no input command` 警告 0 条**。
- **整条链路在「窗口被完全盖住」的状态下手动跑通（22:23~22:24）**：
  道具奖励面板上点「再來一場」`(12.23, 8.34)` → 确认框弹出（「要再進行一場　寶牌雙刃・四人東　の匹配對局嗎?」）
  → 点框里的「確定」`(6.58, 6.71)` → **22:24:38.794 `.lq.Lobby.startUnifiedMatch`**。
  **全程那个置顶遮挡窗一直开着** —— 这证明后台状态下**整条续局链路都是通的**。
- **后台运行实测（2026-11-20 22:0x）**：
  - 帧率：被别的窗口**完全盖住** 0.0 fps → **60.0 fps**；
    **拖到屏幕外** 0.0 fps → **60.0 fps**；切到别的程序（失焦但可见）本来就 60.0 fps；
    **最小化仍是 0.0 fps（无解）**。
  - 一个 1400×900 的置顶窗口把雀魂**整个盖住**之后 75 秒里，
    弃牌数 **22 → 41**，**零** `no input command after clicking` 警告，
    且 bot 的**碰（Pon）**和**大明杠（Daiminkan）**这种多步复合操作都点成功了。
  - 实机命令行里**只有一条** `--disable-features`：
    `TranslateUI,InterestFeedContentSuggestions,CalculateNativeWinOcclusion` —— 合并逻辑生效。
- **Rust 单测 1108 / 1108 全过**（含新增的：随机延迟默认 opt-in、碰/吃除外、反向区间按定值读、配置缺段取默认、续局候选顺序、对话框緊跟「再來一場」、「確定」兜底次数 = 结算画面数、续局默认坐标、**点击预算覆盖最长路径**、**开局信号优先级**、**匹配准备信号与大厅信号互相独立**）。
- **前端单测 109 / 109 全过**；`tsc -b` + `vite build` 通过；`eslint` 改动文件 **0 error 0 warning**；四个语言包 JSON 合法且行尾保持不变。
- **续局坐标是实测值**（2026-11-20，繁体客户端，**两局半庄**结束后用系统级真实鼠标事件逐屏点过去）：
  - 「確定」金色板 bbox `(1414,1056)-(1630,1118)` → 归一化 **(14.59, 8.37)**（两局各自复现，bbox 完全一致）；
  - 「再來一場」蓝色板 bbox `(1170,1054)-(1384,1114)` → 归一化 **(12.23, 8.34)**；
  - 对话框「確定」金色板 bbox `(558,884)-(820,946)` → 归一化 **(6.58, 6.71)**；
  - 画布矩形由 AkagiMS 日志里的 `x_norm → css_x` 换算关系解出
    （`css_x = 69.3134·x_norm + 0.4818`，dpr 1.5 → canvas 物理原点 `(4.4, 216.5)`、尺寸 `1663.5×936.3`）。
- **结算流程实测（完整跑通两次）**：終局积分板 → 禮物盒 → 道具奖励（三屏的「確定」同坐标，第 3 屏多一个「再來一場」）→ 点「再來一場」弹出二次确认框 → 点框里的「確定」→ `startUnifiedMatch` → 下一局。
- **第三局完整重走（16:14~16:20）**：終局积分板（16:14:14.412 点「確定」）→ 一姬的禮物盒（16:14:5x 点「確定」）→ 道具奖励面板（浮汗藥水 +1 / 浮空石 +1 / 迷霧晶塵 +100，16:15:41.339 点「確定」）→ **匹配准备画面**（「寶牌雙刃」规则说明 + 橙色「匹配對局」按钮）。**这三步每一步 uplink 都是 0 帧。**
- **「匹配對局」实测**：16:19:47.654 点那个橙色按钮 → **16:19:48.643 `.lq.Lobby.startUnifiedMatch`**（1 秒后）→ 16:19:53.433 `NotifyMatchGameStart` → 16:20:19.057 東1 开打。**这条路径不需要确认框**，直接开局。归一化坐标 **(10.87, 7.10)**。
- **推翻了「点完三个结算画面就回大厅」这个旧结论**：实测走到的不是大厅，是**匹配准备画面**。
- **`route-4` 的真实含义实测**：「大厅路由被重载」，**不等于**「客户端站在大厅画面上」。两次反例：13:51:49.565（**对局进行中**就重载了一次大厅路由）、16:19:15.565（客户端在匹配准备画面上**静置 3 分 38 秒**后自己重载大厅路由）。已把模块对它的用法从「流程的一步」改成「兜底停手条件」。
- **推翻了「`route-3` 是确认框的信号」这个结论 —— 而它正是旧版模块一场都开不出来的原因**：
  在 v3 自己拉起的 Chromium 里把整条链路逐帧看下来：点「再來一場」时**协议里一个包都没有** —— 无请求、无通知、**无路由变化**，整条链路里 `route-3` **一次都没出现过**；对话框在 wire 上**完全隐形**。
  而点对话框的「確定」，客户端在**同一个毫秒**就发出了 `startUnifiedMatch`（不是旧说明写的「晚约 2 秒」）。
  旧模块等 `route-3` 才去点对话框的「確定」，于是这个「確定」**一次都没被点过**；接着它又去点「確定」—— 那一下落在**对话框背后的道具面板**上，把客户端从对话框底下走掉了。
  **已改为「点完『再來一場』立刻点对话框的『確定』」，不再依赖任何信号**；`route-3` 的含义也改回本来的样子（客户端走到了匹配准备画面）。
- **在 v3 自己拉起的 Chromium 里把整条链路跑通了**（2026-11-20 21:0x）：用 CDP 直接派发 `Input.dispatchMouseEvent`（和模块走的是同一条路径）
  點「確定」→ 禮物盒 → 點「確定」→ 道具奖励面板 → 點「再來一場」→ 弹出「要再進行一場　寶牌雙刃・四人東　の匹配對局嗎?」→ 點框裡的「確定」
  → **21:05:34.684 `.lq.Lobby.startUnifiedMatch`** → **21:05:39.666 `NotifyMatchGameStart`** → 下一局 bot 正常出牌。
  **坐标不用重新校准**：v3 的画布是 1036 × 582 CSS px、`y = 95`、内容 2560 × 1440，
  于是「確定」落在 **(944.7, 636.3)**、「再來一場」落在 **(791.9, 634.3)**、对话框的「確定」落在 **(426.1, 528.9)** —— **三个都在按钮本体内**。
- **结算动画会吃掉点击（这一轮才发现）**：结算画面是带动画的，动画没走完时点下去**完全不响应**。
  所以模块每点一下之后都要等 `step_wait_ms`（默认 2000 ms）落定，并且「確定」的次数上限从 3 放宽到 15、总点击预算从 12 放宽到 36 —— 宁可多点几下，也不要把预算浪费在动画里。
- **「再來一場」会弹二次确认框（这一轮才发现的关键一环）**：15:48:45.523 点「再來一場」→ 对话框弹出、**wire 上 0 帧**；15:50:51.959 点框里的「確定」→ **15:50:54.018 `.lq.Lobby.startUnifiedMatch`**（2.06 秒后）→ 15:50:57.794 `NotifyMatchGameStart`。**真正发出开局请求的是对话框里的那个「確定」**。
- **对话框文案实测**：「要再進行一場　賣牌雙刃・四人東　的匹配對局嗎?」；框里左边金色「確定」、右边蓝色「取消」、右上角 X；客户端此时切到 **route-3**（匹配准备），不是大厅路由。
- **大厅坐标地图实测**（2026-11-20 繁体客户端，用系统级真实鼠标事件把每个面板点开再点关）：顶栏 4 个挂牌、3 张吊牌、底部 7 个导航按钮、段位場列表页返回箭头，全部有归一化坐标，见 §4.6.1。坐标一律用**像素法**测（目测排名面板的 X 偏了 32 物理像素）。
- **大厅误触实测**：
  - 在大厅点「再來一場」坐标 `(12.23, 8.34)` → **完全没反应**（画面不变、wire 无变化）；
  - 在大厅点对话框「確定」坐标 `(6.58, 6.71)` → 落在角色头发上，**完全没反应**；
  - 在大厅点「確定」坐标 `(14.59, 8.37)` → 客户端 0.8 秒后新建 route-2 连接、3.4 秒后 **`route-4 → route-2`**（大厅 → 段位場）、画面切到「牌楼 + 彩虹」场景，35 秒后客户端自己发出 `startUnifiedMatch`（`match_sid 2:55` = 段位場）开了一局。全程 AkagiMS 无动作（bot「已停止」）。
  - 对照：同样在大厅的 90 秒里 wire 上只有心跳、**没有** `startUnifiedMatch` —— 客户端不会自动排队。
- **单局结束画面实测**：東1 打完后那张画面右下角是**蓝色**的「(N) 確認」（抓到的是「(2) 確認」），bbox `(1453,1069)-(1608,1124)`、归一化 `(14.67, 8.46)`，与金色「確定」几乎同位 —— 配置值 `(14.59, 8.37)` 同样落在它上面。**这张画面 AkagiMS 自己就会点**：实测 15:36:41 画面出现 → 15:36:56 发出 `confirmNewRound` → 東2 直接开打。
- **wire 层实测**（逐帧记录 uplink）：点掉这三屏**一个协议包都不发**；`.lq.FastTest.confirmNewRound` 出现在**开局**时而非结算关闭时。
- **路由编号实测**：`.lq.Route.requestRouteChange` 的 `route_id` —— `route-2` = 段位場大厅、`route-3` = 匹配准备、`route-4` = **大厅**、`route-5` = 对局中。模块按 route-3 认对话框、按 route-4 兜底停手。
- **推翻了一个旧结论**：`fetchAccountInfo` **不是**「回到大厅」的信号 —— 它在一局结束后约 24 秒就会发一次（15:45:31.896 `NotifyGameEndResult` → 15:45:56.778），那时候終局积分板还亮着，而且**没有任何路由切换**。上一版说明把它当大厅信号是错的，已改用路由切换。
- **缩放实测**（真实 Chrome 加载 `https://game.maj-soul.com/1/`）：窗口 1600×1000 → canvas CSS 1600×900；窗口 1120×700 → canvas CSS 1120×630（比值正好 0.700）；宽高比恒为 16:9；**绘制缓冲恒为 2560×1440** —— 所以归一化坐标与窗口尺寸无关，不需要图像识别。
- **配置往返正确**：exe 启动日志读到
  `random_delay: RandomDelayConfig { enabled: true, min_ms: 0, max_ms: 3000, exclude_chi_pon: true }`，
  `requeue: RequeueConfig { enabled: true, max_games: 10, result_confirm: [14.59, 8.37], ... }`；
  exe 退出后写回的 `configs/config.toml` 里 `[autoplay.majsoul.random_delay]` 段完整落盘。
- **exe 能启动**：配置加载、MITM 代理、bot manager、game tracker 全部起来，前端资源正常加载。
- **新 UI 确实进了产物**：构建出的 bundle 里能查到 `random_delay_enable` / `random_delay_range` /
  `random_delay_exclude_chi_pon` / `model_server_default` 四个键，以及「随机延迟打牌」「碰/吃除外」「服务器默认」三条中文文案和 `__server_default__` 哨兵值。
- **AkagiMS 与本构建同时运行**时，两个窗口都正常（identifier 改过之后实测）。
- **悬浮窗那一行实机跑通了**（2026-11-21）：把 v3 用 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`
  挂上 CDP，直接驱动悬浮窗页面 —— 读到的行内容是「已打 0 · 剩 20 / 这局后停 / 时间框」；
  点「这局后停」后 `config.toml` 里 `max_games` 由 **20 变成 1**，日志记
  `requeue: stopping after the game in play (budget set to 1)`；
  在时间框里填 `23:30` 后 `config.toml` 里出现 `stop_at = "23:30"`，
  日志记 `requeue: the plan was changed — max_games = 1, stop_at = Some("23:30")`。
- **清除截止时间的一个真 bug 被这套测试揪出来并修掉了**：配置文件是**合并**写回磁盘的，
  而空值在合并时根本不写 —— 所以「清除」只改了内存里的值，磁盘上的 `stop_at` 还在，
  下次启动又回来了。改成用空串表示「没设」，并加了回归单测
  （`src/config/merge.rs::an_empty_value_overwrites_the_one_in_the_file`）。
- 补丁 31/31 文件字节级一致（`git apply` 后逐文件比对）。
- 权重下载源的体积与可用性：全部实测（见 §1.1）；`mortal.pth` 的网络结构用 torch 直接读了出来。

### 未验证（需要你实机跑）

- **随机延迟在真实牌局里的观感**：没有打过一整局实牌，随机延迟只做了配置往返与单元测试层面的验证。
- **模型下拉在真实 key 下的表现**：没有可用的 MJAPI key，`/v3/models` 无 key 返回 403，所以下拉列表的**真实内容**没跑过；UI 逻辑（自动拉取、手填回退、服务器默认项）有单测覆盖。
- **「再來一場」「對話框確定」「匹配對局」按钮在 v3 里的真实点击效果**：**已经在 v3 自己的 Chromium 里手动跑通整条链路了**（见上面「在 v3 自己拉起的 Chromium 里把整条链路跑通了」那一条）。**但新版模块自己的那套逻辑还没在实牌里验证过** —— 需要你重启新版之后再打一局。
- **「匹配對局」作为兜底没有在真实场景里跑过**：那条路（点完三个结算画面 → 停在匹配准备画面 → 点橙色按钮）只在手动实测里走通过一次；「另外三个人都走了 → 「再來一場」消失」这个前提本身还没遇到过。
- **「另外三个人都走了 → 「再來一場」消失」这个前提本身还没遇到过**，所以「匹配對局」那条兜底路（走完三屏 → 停在匹配准备画面 → 点橙色按钮）在真实场景里还没跑过 —— 只在手动实测里走通过一次。
- **「再來一場」的语义已查明一半**：对话框文案写的是「要再進行一場　賣牌雙刃・四人東　的匹配對局嗎?」，点「確定」后客户端发出的是 `startUnifiedMatch {"match_sid":"2:55"}`（`2` = 段位場），即**开一局新的段位場**，而不是「跟上一桌的另外三个人再战」。所以另外三个人都走了的时候这个按钮一样在 —— 「再來一場」消失的情况还没遇到过。
- **「活動」通知弹窗的关闭按钮坐标**：始终没遇到这种弹窗，所以还只能靠 `extra_buttons` 手填。
- 段位场连挂的封号风险。
- **作者的 Discord 上是否仍在分发真权重**：没有 Discord 账号，无法核实（§1.3）。

---

## 10. 与 AkagiMS 的关系

本包是 **Akagi v3**（原版，Apache-2.0 开源）的补丁，**不是** AkagiMS 的补丁。
两者共用同一套 `src/autoplay/` 内核（坐标表、延迟模型、点击合成完全一致），
所以本包在 v3 里跑的效果和 AkagiMS 的「全自動」在行为上等价，
区别只是 v3 走 **chromium 捕获模式**（CDP 直接抓 WebSocket，**不需要装根证书**），
而 AkagiMS 是内嵌 webview。

| | Akagi v3 + 本包 | AkagiMS 全自動 |
|---|---|---|
| 续局 | ✅ | ✅（但需云端 key） |
| 随机延迟（碰/吃除外） | ✅ **新增** | ❌ |
| 界面里自由选模型 | ✅ **新增** | ⚠️ 模型 id 只能手改配置 |
| 需要根证书 | ❌（chromium 模式走 CDP） | ❌ |
| 需要付费 | ❌（云端推理 key 可自备） | ✅（云端推理门禁） |
| 能否换强 AI | ✅（`bot.api` 指向任意兼容端点 / 装外部 bot） | ✅（同一套） |
| 多平台 | 雀魂 / 天凤 / 雀魂都市 | 仅雀魂 |
| 源码 | Apache-2.0 全开源 | 不开源 |

---

## 11. 文件清单

| 文件 | 说明 |
|---|---|
| `akagi/akagi.exe` | 可直接运行的成品（**不要单独移动 dll**） |
| `akagi/WebView2Loader.dll` | WebView2 动态加载器，必须和 exe 同目录 |
| `Akagi-v3增强补丁.patch` | 对上游 v3 的完整补丁（31 个文件） |
| `Akagi-v3增强版使用说明.md` | 本文件 |
| `校准工具/` | 说明书 §4.9 的重新校准步骤要用的一组测量脚本（截图、裁剪、找按钮、点击、翻 wire 日志）；其中 **`cdp.py`** 可以直接对 v3 自己的 Chromium 下手（自动发现随机 CDP 端口、`rect` / `eval` / `shot` / `click`），用法见该目录的 `README.md` |
