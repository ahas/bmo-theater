[English](README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | **简体中文**

# bmo-theater

以单个服务提供平台的视频库:由剧集组成的作品,来源包括作为产出物归档到其自有文件根目录中的视频,以及磁盘上已有的视频
文件库(原地建立索引);每一集都以文件形式通过 HTTP Range 请求流式传输,字幕转换为 WebVTT,并可按需烧录进画面;还提供
播放列表、每位观看者的观看进度和下一部推荐、无需登录即可打开视频的分享链接,以及向公司内指定账户开放视频的授权。所有
这些都保存在相互隔离的两个领域中:各账户自己的领域和组织的领域。此外还包含在控制台侧边栏条目下展示这些内容的控制台
包。`docs/design.md` 说明这个服务是什么,`docs/api.md` 说明它的边界:所有路由、发布的事件、部署的工具、运行的命令以及
设置。

## 测试

`cargo test`(通过 testbed)运行该 crate 唯一的测试 harness `tests/main.rs`,每个测试都在测试套件服务器上使用自己的
PostgreSQL 数据库和自己的文件根目录。覆盖内容包括:从文件名读取编号(`names.rs`)、字幕格式转换(`subtitles.rs`)、
Range 响应(`range.rs`)、进度与下一集(`progress.rs`)、用 ffmpeg 生成的极小视频对媒体库进行规划、扫描、流式传输、
重新扫描和烧录,以及通过上传和路径归档视频(`scan.rs`,未安装 ffmpeg 时跳过)、领域与访问守卫、播放列表与观看位置
(`realms.rs`)、作品列表的排序方式(`orders.rs`)、继续观看与接下来播放(`watching.rs`)、作品的重命名、移动与删除
(`work.rs`)、分享链接(`shares.rs`)、授权(`grants.rs`)、控制台界面(`console.rs`)、OpenAPI 文档(`openapi.rs`),
以及仓库自身的检查(`repository.rs`)。工具的 crate 有自己的 harness `tools/theater/tests/main.rs`,针对一个 theater
服务的替身运行。控制台包的测试位于 `client/tests/`,由 `bun run check` 针对用 Bun 实现的服务替身运行。

## 目录结构

| 文件夹 | 内容 |
|---|---|
| `src/` | 服务及其命令:路由、数据表及其迁移、扫描、烧录、字幕转换器、Range 传输、控制台界面的接线,以及 OpenAPI 文档 |
| `tools/` | 部署时放到工具主机上的 theater 工具:它的 crate、清单、输入 schema 和手册 |
| `client/` | 控制台插件包,自成一个项目 |
| `deploy/` | 服务的用户单元 |
| `docs/` | 说明服务是什么及其边界的权威文档,以及 OpenAPI 文档 |
| `tests/` | 该 crate 唯一的测试 harness,以及各模块共享的内容 |

根目录下有 Cargo 清单、`Cargo.lock`、`testbed.toml`、`install.toml`（在新机器上安装本单元所需的内容）、`rustfmt.toml`、`settings.toml`(编译进二进制的默认值)、
`CLAUDE.md`、`PERSONAL-DATA.md`、`LICENSE`,以及本 README 和它的韩文、日文、中文译本。

## 命令

- `bmo-theater serve` 将数据库迁移到当前 schema,然后持续响应路由,直到收到停止指令。
- `bmo-theater migrate` 将数据库迁移到当前 schema 后退出。
- `bmo-theater health` 在设置所指定的服务回应就绪时以 0 退出。
- `bmo-theater openapi` 输出 OpenAPI 文档,即 `docs/openapi.json` 所保存的内容。
- `bmo-theater console-reach` 以一个 JSON 对象输出控制台如何访问该服务,供部署时的控制台探测使用。
- `bmo-theater tool-env` 写入数据根目录下的 `env/theater.env`,工具通过它找到服务。

设置的读取顺序为:二进制内置的 `settings.toml`,然后是部署时放在发布目录旁的 `settings.toml` 和 `settings.local.toml`
(数据根目录下的 `env/theater.toml`),最后是环境变量 `BMO_THEATER_<TABLE>__<KEY>`(`docs/api.md`,「Settings」)。
