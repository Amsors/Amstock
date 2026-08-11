# Amstock 家用物资管理

Amstock 是对个人电子元件、工具、家用物资的轻量物资管理系统。

- 前端：Vue 3、TypeScript、Vite
- 后端：Rust、Axum、SQLx
- 数据库：SQLite
- 图片：文件系统存储

## 主要功能

### 检索与查看

- 首页聚焦于检索、创建和结果列表。
- 支持按名称、描述或完整编号模糊检索。
- 可以选择是否在结果中包含已删除元素。
- 结果显示编号、类型、数量、单位、父容器、描述和缩略图。
- 收纳树页面以层级结构展示孤立元素、容器及其后代。

### 创建与编辑

- 自动分配六位序列号。
- 可以创建物品或容器。
- 可以修改类型、标记、名称、描述、数量、单位和父容器。
- 非空容器不能直接改为物品。
- 父容器输入框只有在用户输入内容后才查询和展示候选容器。
- 第一段和第二段标记可以通过鼠标从完整合法值列表中选择。

### 连续添加

“继续添加”会先保存当前条目，然后保持窗口打开并准备下一条：

- 保留元素类型、第一段标记和第二段标记。
- 清空名称、描述、单位、父容器和图片。
- 数量恢复为默认值 `1`。
- 第三段数字可以选择保留、清零或自增 `1`；`99` 自增后回到 `00`。
- 第三段数字的连续录入策略保存在浏览器 Cookie 中。

### 图片

- 可以在创建时上传图片，也可以之后添加、替换或移除。
- 每个元素最多一张图片。
- 支持 JPEG、PNG、WebP 和 GIF。
- 请求体上限为 10 MiB。
- 以六位序列号作为文件名保存在图片目录中，不存入数据库；MIME 类型保存在数据库中。

### 移除与恢复

删除采用软删除，编号不会再次分配。普通物品或空容器可以直接移除。

移除非空容器时必须明确选择一种处理方式：

1. 将直接子元素移动到上一级容器；没有上一级时设为孤立元素。
2. 将直接子元素移动到指定的已有容器。
3. 递归软删除全部后代。执行前会以 DFS 顺序展示完整删除清单，并要求用户确认。

已删除元素可以恢复。如果原父容器仍然可用，会恢复原父级关系；否则恢复为孤立元素。递归删除的后代需要分别恢复。

## 页面布局

- **检索与创建**：默认首页，提供搜索、创建、编辑、删除和恢复入口。
- **收纳树**：查看容器与物品的嵌套结构。
- **编号映射**：维护第一段和第二段标记的名称映射。

界面保持简洁朴素设计，同时适配桌面与手机布局。

## 开发与维护

### 项目结构

```text
Amstock/
├── backend/
│   ├── src/main.rs          # 服务启动与全局配置
│   ├── src/routes.rs        # API、层级、删除恢复和图片逻辑
│   ├── src/models.rs        # 请求、响应和数据库模型
│   ├── migrations/          # SQLx 数据库迁移
│   └── data/                # 本地数据库与图片（不提交到 Git）
├── frontend/
│   ├── src/App.vue          # 页面入口
│   ├── src/components/      # 表单、列表、树、映射和 Toast
│   ├── src/api.ts           # 后端 API 客户端
│   ├── src/types.ts         # 前端数据类型
│   ├── src/uiConfig.ts      # 可调 UI 参数
│   └── src/styles.css       # 桌面与手机样式
└── README.md
```

### 启动项目

需要 Node.js 20.19+ 与 Rust 1.85+。

首次安装前端依赖：

```bash
cd frontend
npm install
```

启动后端（开发环境需要设置单用户密码；HTTP 下关闭 Secure Cookie）：

```bash
cd backend
AMSTOCK_PASSWORD='替换为至少八位的开发密码' \
AMSTOCK_COOKIE_SECURE=false \
cargo run
```

要连接真实打印机，将启动模式改为 `printer`：

```bash
cd backend
cargo run -- --print-mode printer --printer-host 192.168.31.114
```

另开一个终端启动前端：

```bash
cd frontend
npm run dev
```

默认访问 <http://localhost:43691>。Vite 开发服务器会把 `/api` 和 `/images` 代理到 `http://127.0.0.1:3000`。

### 数据与配置

默认路径：

- SQLite：`backend/data/amstock.db`
- 图片目录：`backend/data/images/`

后端环境变量：

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `AMSTOCK_DATABASE_URL` | `sqlite://data/amstock.db` | SQLite 连接地址 |
| `AMSTOCK_IMAGE_DIR` | `data/images` | 图片文件目录 |
| `AMSTOCK_BIND` | `127.0.0.1:3000` | 后端监听地址 |
| `AMSTOCK_USERNAME` | `admin` | 唯一允许登录的用户名 |
| `AMSTOCK_PASSWORD` | 无 | 登录密码，至少 8 个字符 |
| `AMSTOCK_PASSWORD_HASH` | 无 | Argon2id PHC 哈希；设置后优先于明文密码 |
| `AMSTOCK_COOKIE_SECURE` | `true` | 是否只通过 HTTPS 发送会话 Cookie；本地 HTTP 开发设为 `false` |
| `AMSTOCK_SESSION_TTL_HOURS` | `720` | 登录会话有效期（小时） |

服务不提供注册和多用户管理。会话保存在后端内存中，因此后端重启后需要重新登录。

### 标签打印配置

后端通过 `printer/amstock_printer.py` 的 stdin/stdout JSON 接口调用 Python。Python
虚拟环境尚未创建时，先按 `printer/README.md` 安装依赖。直接 `cargo run` 时默认生成
PNG 并调用桌面看图程序实时打开；容器镜像强制关闭系统看图程序，适合无头环境。

容器部署主要使用以下环境变量：

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `AMSTOCK_PRINTER_ENABLED` | `false` | `true` 连接真实打印机；`false` 仅生成 PNG 预览 |
| `AMSTOCK_PRINTER_HOST` | `192.168.31.114` | 打印机 IP 或主机名 |
| `AMSTOCK_PRINTER_PORT` | `9100` | RAW TCP 端口 |
| `AMSTOCK_PRINTER_TIMEOUT` | `3` | 打印机连接超时秒数 |

非容器启动也可以使用这些环境变量。命令行参数具有更高优先级：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--print-mode` | `preview` | `preview` 生成并打开 PNG；`printer` 连接真实打印机 |
| `--printer-python` | `printer/.venv/bin/python` | Python 解释器路径 |
| `--printer-script` | `printer/amstock_printer.py` | Python 桥接脚本路径 |
| `--label-preview-dir` | `backend/data/label-previews` | 预览 PNG 输出目录 |
| `--no-open-label-preview` | 关闭 | 只生成预览图，不调用系统看图程序 |
| `--printer-host` | `192.168.31.114` | 打印机 IP 或主机名 |
| `--printer-port` | `9100` | RAW TCP 端口 |
| `--printer-timeout` | `3` | 打印机连接超时秒数 |
| `--printer-no-cut` | 关闭 | 打印后不切纸 |

B1/B2 使用容器的未删除直接子级。Rust 在调用 Python 前按完整编号的类别字母、
BB、CC 和六位序列号依次升序排列；Python 保持传入顺序渲染。

数据库结构由 `backend/migrations/` 中的 SQLx 迁移管理。后端启动时自动执行尚未应用的迁移；首次升级已有数据库时，初始迁移会登记现有表而不会清空业务数据。

## Docker 部署

生产部署由一个应用容器和一个 Caddy 容器组成。Caddy 是唯一对公网开放端口的服务，负责 `amstock.amsors.top` 的 HTTPS 和 Vue 静态文件；Rust API 仅在 Compose 内网监听。

- `docker-compose.yaml` 是生产配置，只从 Docker Hub 拉取镜像，服务器不会构建。
- `docker-compose.dev.yaml` 是本地构建覆盖层，不应单独使用。

Ubuntu 24.04 上准备目录并配置：

```bash
cp .env.example .env
mkdir -p backups
chmod 700 backups
chmod 600 .env
```

编辑 `.env`，至少替换 `AMSTOCK_PASSWORD`。域名默认已经是 `amstock.amsors.top`。
如需使用网络打印机，把 `AMSTOCK_PRINTER_ENABLED` 改成 `true`，并填写跳板网络中
可达的 `AMSTOCK_PRINTER_HOST` 与 `AMSTOCK_PRINTER_PORT`。

将域名的 A/AAAA 记录直接指向服务器，并确保防火墙允许 TCP 80、TCP 443 和 UDP 443，
然后只拉取并启动 Docker Hub 镜像：

```bash
docker compose -f docker-compose.yaml pull
docker compose -f docker-compose.yaml up -d
docker compose ps
```

升级时重复执行 `pull` 和 `up -d` 即可。若需要回退或固定版本，可在 `.env` 中把
`AMSTOCK_IMAGE_TAG` 设为 Actions 生成的 `sha-<提交短哈希>` 或版本号（例如 `v1.0.0`）。

本地需要验证 Docker 构建时使用两个 Compose 文件：

```bash
docker compose -f docker-compose.yaml -f docker-compose.dev.yaml up -d --build
```

如果不希望在 `.env` 中保存明文密码，可以先交互式生成 Argon2id 哈希：

```bash
docker compose run --rm --no-deps web \
  caddy hash-password --algorithm argon2id
```

把输出放入单引号包裹的 `AMSTOCK_PASSWORD_HASH`，并清空 `AMSTOCK_PASSWORD`。

Caddy 会自动申请并续期 HTTPS 证书。业务数据位于 Docker 命名卷 `amstock-data`，删除或重建应用容器不会删除该卷。不要运行 `docker compose down -v`，除非明确希望删除业务数据和 Caddy 证书数据。

### 与宿主机 Nginx 共用服务器

如果服务器上的 Nginx 还承载其他业务，不要让容器绑定公网的 80/443 端口。
使用 Nginx 覆盖配置后，容器内 Caddy 只监听纯 HTTP，并且只映射到宿主机回环地址
`127.0.0.1:8080`：

```bash
docker compose -f docker-compose.yaml -f docker-compose.nginx.yaml pull
docker compose -f docker-compose.yaml -f docker-compose.nginx.yaml up -d
docker compose -f docker-compose.yaml -f docker-compose.nginx.yaml ps
```

如需换一个未占用的回环端口，在 `.env` 中修改
`AMSTOCK_NGINX_UPSTREAM_PORT`。之后在宿主机 Nginx 对应域名的 HTTPS
`server` 块中加入：

```nginx
client_max_body_size 10m;

location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

域名证书、HTTP 到 HTTPS 跳转仍由宿主机 Nginx 管理。修改后先验证再平滑重载：

```bash
sudo nginx -t
sudo systemctl reload nginx
```

使用这种部署方式时，后续每次 `pull`、`up`、`down` 或 `logs` 都应同时传入这两个
Compose 文件；不要只运行基础 `docker-compose.yaml`，否则会重新尝试占用 80/443。

查看日志：

```bash
docker compose logs -f app web
```

### GitHub Actions 与 Docker Hub 配置

仓库中的 `.github/workflows/docker-publish.yaml` 会构建 Dockerfile 的 `backend` 和
`web` 两个目标，并发布到同一个 Docker Hub 仓库 `amsors/amstock`：

- `master` 分支：`backend-latest`、`web-latest`，以及两份 `*-sha-<短哈希>` 镜像。
- `v*` Git 标签：例如 `backend-v1.0.0`、`web-v1.0.0`。
- Pull Request：只验证两个镜像能够构建，不登录或推送 Docker Hub。

首次使用前完成以下一次性配置：

1. 在 Docker Hub 的 `amsors` namespace 下创建公开或私有仓库 `amstock`。
2. 在 Docker Hub 的账户安全设置中创建具有 Read & Write 权限的 Access Token。
3. 打开 GitHub 仓库的 **Settings → Secrets and variables → Actions → New repository secret**，添加：
   - `DOCKERHUB_USERNAME`：Docker Hub 用户名（通常为 `amsors`）。
   - `DOCKERHUB_TOKEN`：上一步生成的 Access Token，不要填写登录密码。
4. 推送到 `master`，或在 GitHub 的 **Actions → Build and publish Docker images → Run workflow** 手动执行。

Docker Hub 仓库如果设为私有，部署服务器还需要先执行 `docker login`，再运行
`docker compose pull`。

### 将当前非容器数据迁入

先停止当前后端的写入，然后在项目根目录将 `backend/data` 打成兼容的导出包：

```bash
AMSTOCK_DATA_DIR=backend/data ./deploy/amstock-export backups/amstock-initial.tar.gz
```

首次启动新容器前，或者先执行 `docker compose stop app`，再导入：

```bash
docker compose run --rm --no-deps app amstock-import /backups/amstock-initial.tar.gz
docker compose up -d
```

导入完成后，应用启动时会自动执行数据库迁移。

### 容器数据导出与恢复

为保证 SQLite 与图片目录完全一致，导出和导入时短暂停止应用。导出包保存在宿主机的 `backups/`：

```bash
docker compose stop app
docker compose run --rm --no-deps app amstock-export
docker compose start app
```

也可以指定文件名：

```bash
docker compose run --rm --no-deps app \
  amstock-export /backups/amstock-manual.tar.gz
```

恢复指定导出包：

```bash
docker compose stop app
docker compose run --rm --no-deps app \
  amstock-import /backups/amstock-manual.tar.gz
docker compose start app
```

导入工具会先检查压缩包和 SQLite 完整性，并将导入前的数据自动导出为 `backups/amstock-pre-import-*.tar.gz`。导出包包含数据库和原始图片，不包含可重新生成的标签预览。

### 构建与测试

```bash
# 后端格式检查与测试
cd backend
cargo fmt --check
cargo test --locked

# 前端类型检查与生产构建
cd frontend
npm run build
```



## 项目设定

### 数据

系统中有两种元素：

- **物品**：电子元件、工具、生活物资等实际需要管理的对象。
- **容器**：箱子、抽屉、零件盒等收纳位置。

容器可以包含物品或其他容器。每个元素最多只有一个父容器，也可以不属于任何容器。父级必须是未删除的容器。

每个元素包含以下数据：

| 字段 | 说明 |
| --- | --- |
| 类型 | `item`（物品）或 `container`（容器） |
| 完整编号 | 系统生成的 `A-BB-CC-DDDDDD` 格式编号 |
| 名称 | 必填，支持中文 |
| 描述 | 可选，用于记录规格、用途或备注 |
| 数量 | 非负浮点数，新建时默认为 `1` |
| 单位 | 可选字符串，例如个、盒、米 |
| 父容器 | 可选，只能指向一个未删除容器 |
| 图片 | 可选，每个元素最多一张 |
| 删除时间 | 用于软删除与恢复 |

### 编号规则

完整编号格式为：

```text
A-BB-CC-DDDDDD
```

示例：

```text
M-01-85-862390
A-00-67-425678
I-34-00-000000
```

各段约束如下：

- 第一段是一位英文字母。
- 第二段是 `00` 到 `99` 的两位数字。
- 第三段是 `00` 到 `99` 的两位数字。
- 最后一段是六位全局序列号，也是元素的主键。

六位序列号由物品和容器共享，从 `000000` 开始连续递增。软删除后不会回收或复用旧序列号；恢复元素时仍使用原序列号。

前三段是用户可编辑的标记，系统不假设这些标记的具体业务含义。

### 标记映射

可为第一段字母配置名称，并为每个字母维护一张独立的第二段数字映射表。例如，同为 `03` 的第二段数字，在不同字母下可以具有不同名称。

- 一个标记值最多对应一个名称。
- 名称可以留空，此时作为未配置映射显示。
- 第三段数字不维护名称映射。
