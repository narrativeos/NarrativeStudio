# NarrativeStudio 开发文档

> 版本：v0.1.0
> 日期：2026-09-14
> 状态：Draft
> 前置文档：[产品设计文档](./product-design.md)

---

## 目录

1. [项目初始化](#1-项目初始化)
2. [开发环境搭建](#2-开发环境搭建)
3. [编码规范](#3-编码规范)
4. [Tauri Command API 合约](#4-tauri-command-api-合约)
5. [前端开发指南](#5-前端开发指南)
6. [后端开发指南](#6-后端开发指南)
7. [测试策略](#7-测试策略)
8. [构建与发布](#8-构建与发布)
9. [调试指南](#9-调试指南)
10. [贡献指南](#10-贡献指南)

---

## 1. 项目初始化

### 1.1 从零创建 Workspace

```bash
# 1. 创建 Rust workspace
mkdir NarrativeStudio && cd NarrativeStudio
cargo init --name narrative-studio
mkdir -p crates/{studio-core,studio-import,studio-analysis,studio-storage,studio-report,studio-app}

# 2. 初始化各 crate
for crate in studio-core studio-import studio-analysis studio-storage studio-report; do
  cargo new crates/$crate --lib
done

# 3. 初始化 Tauri 应用（studio-app）
cd crates/studio-app
npm create tauri-app@latest . -- --template react-ts --manager pnpm
cd ../..

# 4. 配置 workspace root Cargo.toml
```

**Workspace root `Cargo.toml`：**

```toml
[workspace]
resolver = "2"
members = [
    "crates/studio-core",
    "crates/studio-import",
    "crates/studio-analysis",
    "crates/studio-storage",
    "crates/studio-report",
    "crates/studio-app/src-tauri",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/narrativeos/NarrativeStudio"

[workspace.dependencies]
# 内部 crates
studio-core = { path = "crates/studio-core" }
studio-import = { path = "crates/studio-import" }
studio-analysis = { path = "crates/studio-analysis" }
studio-storage = { path = "crates/studio-storage" }
studio-report = { path = "crates/studio-report" }

# 外部依赖（统一版本）
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
tokio-util = "0.7"
duckdb = "1"
reqwest = { version = "0.12", features = ["json", "stream"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
sha2 = "0.10"
futures = "0.3"
regex = "1"
pulldown-cmark = "0.13"
```

### 1.2 前端初始化

```bash
cd crates/studio-app

# Tauri create 已生成 React + TS 模板，补充依赖
pnpm add -D tailwindcss postcss autoprefixer
npx tailwindcss init

# 目录结构
mkdir -p src/{components,pages,lib}
```

> 前端**不引入**状态管理库（Zustand 等）与图表库（D3 等）：状态用 React Hooks +
> `invoke` 直连（§5.3），图表手写 SVG（§5.4）。

### 1.3 目录结构总览

```
NarrativeStudio/
├── Cargo.toml                      # Workspace root
├── crates/
│   ├── studio-core/                # 核心类型（无业务逻辑）
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── project.rs
│   │       ├── document.rs
│   │       ├── entity.rs           # Entity（含 stable_id）/ EntityStat
│   │       ├── noun_signal.rs
│   │       ├── analysis.rs
│   │       ├── concern.rs
│   │       ├── knowsphere.rs       # NarratSphere .knowsphere.json v1.0 契约镜像 + 自检
│   │       ├── report.rs
│   │       └── error.rs
│   │
│   ├── studio-import/              # TraceView 结果解析
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── semantic.rs         # semantic_result.json（blocks/tokens/entities）
│   │       ├── popo.rs             # popo_result.json（目录树）
│   │       ├── term.rs             # term_result.json（领域术语）
│   │       └── enrich.rs           # 三者合并增强
│   │
│   ├── studio-analysis/            # 分析引擎（全部确定性规则，无 LLM）
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── hash.rs             # document_hash / ANALYSIS_VERSION（缓存键）
│   │       ├── segment.rs  text.rs  lexicon.rs  calendar.rs
│   │       ├── t0.rs  t1.rs        # 文本统计 / 结构统计
│   │       ├── plot.rs  pacing.rs  conflict.rs
│   │       ├── theme.rs  setting.rs  structure.rs  timeline.rs
│   │       ├── dialogue.rs  cliche.rs  explicit.rs  voice.rs
│   │       ├── checklist.rs        # 最终审查清单：只汇总其余维度的数字，不测量
│   │       └── assessment.rs       # 规则评估与建议
│   │
│   ├── studio-storage/             # DuckDB 持久化
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── migration.rs        # 版本化迁移（当前至 v13：entity_registry）
│   │       ├── project_repo.rs
│   │       ├── document_repo.rs    # Appender 批量写入 + 稳定 ID 落库
│   │       ├── entity_registry.rs  # 稳定实体身份注册表 + entity_stats_for_project
│   │       ├── analysis_repo.rs    # 按 (project, dimension, input_hash) 的结果缓存
│   │       └── report_repo.rs
│   │
│   ├── studio-report/              # 报告生成
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── markdown.rs         # Markdown 报告
│   │       └── knowsphere.rs       # .knowsphere.json 渲染器（NarratSphere 导出）
│   │
│   └── studio-app/                 # Tauri 应用
│       ├── package.json
│       ├── src/                    # 前端 (React，无状态库/图表库)
│       │   ├── main.tsx  App.tsx  types.ts
│       │   ├── pages/              # Home / Analysis / Compare
│       │   ├── components/         # 卡片与手写 SVG 图表
│       │   └── lib/                # format / graph / radar / checklist（纯函数，可单测）
│       └── src-tauri/              # Tauri 后端入口
│           ├── Cargo.toml
│           ├── tauri.conf.json
│           ├── tests/              # 导入→分析→报告 集成测试
│           └── src/
│               ├── main.rs
│               ├── lib.rs          # AppState + 启动 + invoke_handler
│               └── commands/
│                   ├── mod.rs
│                   ├── project.rs  # list / create / delete
│                   ├── import.rs   # import_project（spawn_blocking + 进度事件）
│                   ├── analyze.rs  # analyze_project(_inner) / 缓存 / 文档读取
│                   └── report.rs   # report_* / export_knowsphere
│
├── docs/
│   ├── product-design.md
│   ├── development.md              # 本文档
│   └── README.md
├── .github/
│   └── workflows/
│       └── ci.yml
├── .gitignore
├── LICENSE
└── README.md
```

---

## 2. 开发环境搭建

### 2.1 系统依赖

| 平台 | 依赖 |
|------|------|
| macOS | Xcode Command Line Tools, Rust (rustup), Node.js 20+, pnpm 9+ |
| Windows | Visual Studio Build Tools, Rust, Node.js 20+, pnpm 9+ |
| Linux | build-essential, libwebkit2gtk-4.1-dev, libgtk-3-dev, Rust, Node.js 20+, pnpm 9+ |

```bash
# macOS 快速安装
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install node pnpm
```

### 2.2 日常开发命令

```bash
# 开发模式（热重载）
pnpm --filter studio-app tauri dev

# 仅前端开发（浏览器中，无 Tauri IPC）
cd crates/studio-app && pnpm dev

# 仅后端测试
cargo test --workspace

# 格式化
cargo fmt --workspace
cd crates/studio-app && pnpm format

# Lint
cargo clippy --workspace -- -D warnings
cd crates/studio-app && pnpm lint

# 构建（开发）
pnpm --filter studio-app tauri build --debug

# 构建（发布）
pnpm --filter studio-app tauri build
```

### 2.3 环境变量

| 变量 | 用途 | 默认值 |
|------|------|--------|
| `STUDIO_DATA_DIR` | 数据存储目录 | `~/.NarrativeStudio/` |
| `RUST_LOG` | 日志级别 | `info` |

> 分析全部为确定性规则计算，**没有任何 LLM 调用**，因此不存在 LLM 相关环境变量。

---

## 3. 编码规范

### 3.1 Rust 规范

| 规则 | 说明 |
|------|------|
| 格式化 | `cargo fmt` 强制，CI 检查 |
| Lint | `cargo clippy -D warnings`，零警告 |
| 错误处理 | 使用 `thiserror` 定义错误类型，禁止 `unwrap()`（测试除外） |
| 异步 | 所有 I/O 操作使用 `tokio`，CPU 密集用 `spawn_blocking` |
| 文档 | 所有 `pub` 项必须有 `///` 文档注释 |
| 测试 | 每个模块至少一个单元测试，关键路径有集成测试 |
| 命名 | `snake_case` 函数/变量，`PascalCase` 类型，`SCREAMING_SNAKE` 常量 |
| 模块 | 每个文件一个主要类型/功能，`mod.rs` 只做 re-export |

**错误类型模式：**

```rust
// studio-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StudioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),

    #[error("Import error: {0}")]
    Import(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, StudioError>;
```

### 3.2 TypeScript 规范

| 规则 | 说明 |
|------|------|
| 格式化 | Prettier（`pnpm format`） |
| Lint | ESLint + `@typescript-eslint`，零警告 |
| 类型 | 严格模式（`strict: true`），禁止 `any` |
| 组件 | 函数组件 + Hooks，禁止 class 组件 |
| 状态 | React Hooks（`useState`/`useEffect`）+ `invoke` 直连，不引入全局状态库 |
| 命名 | `PascalCase` 组件，`camelCase` 函数/变量，`UPPER_SNAKE` 常量 |
| 导入 | 使用 `@/` 别名指向 `src/` |

### 3.3 Git 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
<type>(<scope>): <subject>

[body]

[footer]
```

| type | 用途 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修复 |
| `docs` | 文档 |
| `refactor` | 重构（不改变行为） |
| `perf` | 性能优化 |
| `test` | 测试 |
| `chore` | 构建/工具 |

**示例：**
```
feat(analysis): add T0 word frequency statistics task

- Implement HashMap-based word counting
- Add unit tests with sample TraceView data
- Wire into AnalysisOrchestrator pipeline

Closes #42
```

---

## 4. Tauri Command API 合约

### 4.1 Command 列表

Tauri Commands 按领域分组在 `studio-app/src-tauri/src/commands/`（`project.rs` / `import.rs` / `analyze.rs` / `report.rs`），`lib.rs` 只保留应用状态与启动；前端通过 `invoke` 调用（参数用 camelCase，Tauri 自动映射到 Rust 的 snake_case）。

| Command | 参数 | 返回 | 说明 |
|---------|------|------|------|
| `list_projects` | - | `Vec<ProjectSummary>` | 列出所有项目 |
| `create_project` | `{ name }` | `Project` | 创建项目 |
| `delete_project` | `{ project_id }` | - | 删除项目及其级联数据 |
| `import_project` | `{ project_path }` | `Project` | 导入 TraceView 项目目录（`spawn_blocking` + `import-progress` 事件） |
| `analyze_project` | `{ project_id, force? }` | `ProjectAnalysis` | T0 + T1 + 情节 + 节奏 + 冲突 + 主题 + 场景/设定 + 陈词滥调 + 对话/叙事比例 + 显式语言 + 故事结构 + 时间线 + 作者声音 + 最终审查清单 + 评估；命中缓存时 `cached: true`，`force: true` 强制重算（见 §6.5） |
| `analyze_projects` | `{ project_ids }` | `Vec<ProjectAnalysis>` | 多项目对比，始终读缓存 |
| `analyze_t0` / `analyze_t1` | `{ file_path }` | `T0Stats` / `T1Stats` | 直接分析 `semantic_result.json`，调试用，不走缓存 |
| `list_documents` | `{ project_id }` | `Vec<(doc_id, title)>` | 项目下的文档 |
| `load_document_cmd` | `{ doc_id }` | `DocumentData` | 按 ID 载入文档 |
| `report_generate` | `{ project_id }` | `ReportMeta` | 生成 Markdown 报告并入库（复用分析缓存，见 §6.6） |
| `report_export` | `{ report_id, path }` | `path` | 把已入库的报告写到指定路径并记录 |
| `report_export_html` | `{ report_id, path }` | `path` | 把已入库报告的 Markdown 现渲染成打印就绪 HTML 写盘并记录路径（A4 排版，浏览器 ⌘P 存 PDF，见 §6.6） |
| `export_knowsphere` | `{ project_id, path }` | `path` | 渲染 `.knowsphere.json`（供 NarratSphere 消费）、按 schema 契约自检后写入指定路径（见 product-design.md §Knowsphere 导出契约） |

> 上表是**已实现**的命令面。设计稿中的异步任务合约（`analyze_cancel` / `analyze_status` / `settings_get` / `settings_update`）尚未实现：当前分析命令同步返回结果（用 `async fn` + `spawn_blocking` 保证不阻塞 UI 线程），没有 `TaskId` + 事件流。

### 4.2 事件合约

后端通过 `tauri::Emitter` 向前端发送事件：

| 事件名 | Payload | 触发时机 |
|--------|---------|---------|
| `analysis:progress` | `{ task_id, current, total, label }` | 每完成一个子任务 |
| `analysis:result` | `{ task_id, dimension, data }` | 单个维度分析完成 |
| `analysis:complete` | `{ task_id, summary }` | 全部分析完成 |
| `analysis:error` | `{ task_id, dimension?, error }` | 分析出错 |
| `import:progress` | `{ current, total, file_name }` | 导入进度 |

### 4.3 类型定义（前后端共享）

前端类型定义在 `crates/studio-app/src/types/` 中，与 Rust 类型通过 serde 对齐：

```typescript
// src/types/project.ts
export interface Project {
  project_id: string;
  name: string;
  description?: string;
  genre?: string;
  language: string;
  created_at: string;
  updated_at: string;
}

// src/types/analysis.ts
export interface AnalysisProgress {
  task_id: string;
  current: number;
  total: number;
  label: string;
}

export interface AnalysisResult {
  task_id: string;
  dimension: string;
  data: Record<string, unknown>;
}

export type AnalysisStatus = 'idle' | 'loading' | 'partial' | 'complete' | 'error';
```

### 4.4 Command 实现模板

```rust
// studio-app/src-tauri/src/commands/report.rs — 典型形态：重活放阻塞线程池，
// 数据库锁只在读写时持有，命令统一返回 Result<T, String>。
use crate::AppState;
use tauri::State;

#[tauri::command]
pub(crate) async fn export_knowsphere(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<String, String> {
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    // 复用与页面相同的分析（命中缓存则不重算）。
    let analysis = analyze_project_inner(&state, &project_id, false)?;
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let entities = entity_stats_for_project(&storage.conn, pid)
        .map_err(|e| e.to_string())?;
    // …渲染 → doc.validate() → 通过后写文件…
    Ok(path)
}
```

---

## 5. 前端开发指南

### 5.1 页面结构

| 页面 | 组件 | 说明 |
|------|------|------|
| 项目列表 | `pages/Home.tsx` | 所有项目卡片，新建/导入/删除 |
| 分析详情 | `pages/Analysis.tsx` | 单项目 14 维分析视图 + 报告/知识球导出 |
| 多项目对比 | `pages/Compare.tsx` | 选择多个项目对比 |

> 页面切换由 `App.tsx` 内部状态驱动，未引入路由库；没有设置页。

### 5.2 组件层级

```
App
├── Layout
│   ├── Sidebar (导航)
│   └── Content
│       ├── ProjectListPage
│       ├── ProjectDetailPage
│       │   ├── SummaryBar (项目摘要)
│       │   ├── ChartGrid
│       │   │   ├── RadarChart
│       │   │   ├── BarChart
│       │   │   ├── PacingCurve
│       │   │   ├── EntityPie
│       │   │   ├── WordCloud
│       │   │   ├── Heatmap
│       │   │   └── DiagnosticCards
│       │   └── ConcernList
│       ├── ComparePage
│       │   ├── ProjectSelector
│       │   ├── OverlaidRadar
│       │   ├── GroupedBars
│       │   ├── MultiLineChart
│       │   ├── StackedBars
│       │   ├── DiagnosticTable
│       │   ├── SideBySideHeatmaps
│       │   └── SideBySideWordClouds
└── Toast / Modal
```

### 5.3 状态管理

无全局状态库。页面数据在组件内用 `useState`/`useEffect` 直接 `invoke` 获取，跨页面只传 `projectId`：

```tsx
// pages/Analysis.tsx —— 实际模式：参数变化即重新取数，可取消竞态
const [data, setData] = useState<ProjectAnalysis | null>(null);
const [reload, setReload] = useState(0);
const [force, setForce] = useState(false);

useEffect(() => {
  let cancelled = false;
  setLoading(true);
  invoke<ProjectAnalysis>("analyze_project", { projectId, force })
    .then((r) => { if (!cancelled) setData(r); })
    .catch((e) => { if (!cancelled) setError(String(e)); })
    .finally(() => { if (!cancelled) setLoading(false); });
  return () => { cancelled = true; };
}, [projectId, reload, force]);
```

导出类操作（报告 / 知识球）同理：`invoke` + `plugin-dialog` 的 `save()`，
后端负责落盘，前端不需要文件系统权限。

### 5.4 图表组件规范

- 所有图表使用 **SVG 渲染**（非 Canvas），支持响应式
- **不引入图表库**（D3 / Chart.js 等）：组件直接手写 SVG 元素；极坐标、比例尺等
  几何计算放在 `src/lib/`（如 `radar.ts`、`graph.ts`），与 JSX 分离以便用 Vitest 单测
- **布局必须确定性**：同一份输入每次画出同一张图。力导向一类迭代布局每次收敛结果不同，
  既无法复现也无法断言，因此不使用；关系图用按提及数排序的环形布局代替
- 画布尺寸与布局常数放在 `src/lib/`（如 `GRAPH_BOX`），使「标签不溢出画布」这类性质可被测试断言
- 每个图表组件接收数据 + 可选的 `size` / `color` props，数据为空时返回
  「暂无数据」占位而不是渲染空坐标系
- 颜色分两类，**组件里不允许出现裸色号**（`lib/theme.test.ts` 会扫源码把关）：
  - **状态色**走语义 token（`bg-bad` / `text-warn` / `border-line` …），随主题切换；
    曲线下的色带用 `fill={color} fillOpacity` 从曲线自身派生，不再另写一份 rgba
  - **身份色**（词性、章节、某条曲线是哪一维）集中在 `index.css` 的 `@theme static` 块，
    两套主题共用同一色值——它们说明的是「这是哪个东西」，不是「这东西好不好」；
    SVG 属性用 `var(--color-…)` 读取，`static` 保证编译器不会因「没有 class 引用」而丢弃
- 多项目对比目前**没有按项目着色**：表格直接写项目名，因此也不存在 A/B/C/D/E 一套系列色

### 5.5 主题（浅色 / 暗色）

主题只由 `<html data-theme>` 决定，颜色全部住在 `index.css`：

| 位置 | 职责 |
|------|------|
| `index.css` `@theme inline` | 语义 token → 主题变量（`--line`、`--bad`…），两套主题各给一份值 |
| `index.css` `@theme static` | 身份色，主题无关 |
| `lib/theme.ts` | 纯逻辑：`resolveTheme` / `readChoice` / `writeChoice` / `applyTheme` |
| `App.tsx` | 记住用户选择，监听 `prefers-color-scheme` 变化并重绘 |
| `index.html` 内联脚本 | 首帧前定色，避免浅色用户看到暗色闪一下 |
| `components/Sidebar.tsx` | 侧栏底部「主题」三段控件：跟随系统 / 浅色 / 暗色 |

- 默认**跟随系统**；选择存在 `localStorage` 的 `narrativestudio.theme`
- 暗色值就是引入 token 之前那批 Tailwind 工具类的值，因此改造本身**没有改动暗色的任何一个像素**
- `lib/theme.test.ts` 除扫描裸色号外，还断言两套主题都定义齐了语义 token、
  正文/次要文字/强调色与五种状态色在两种底上都不低于 WCAG AA（4.5:1），
  徽标底纹上的文字不低于 3:1（暗色徽标历来约 3.4:1）
- 导出 PDF 用的打印 HTML（`studio-report::html`）自带纸白底样式，不参与主题切换

---

## 6. 后端开发指南

### 6.1 分析任务实现模板

```rust
// studio-analysis/src/tasks/t0_stats.rs
use studio_core::analysis::{AnalysisDimension, AnalysisResult};
use studio_core::document::DocumentData;
use std::collections::HashMap;

/// T0: 词频统计
pub fn word_frequency(doc: &DocumentData) -> AnalysisResult {
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for block in &doc.blocks {
        for token in &block.tokens {
            let word = token.text.to_lowercase();
            *freq.entry(word).or_insert(0) += 1;
        }
    }
    let top_100: Vec<_> = freq.iter()
        .take(100)
        .map(|(w, c)| serde_json::json!({ "word": w, "count": c }))
        .collect();

    AnalysisResult {
        dimension: AnalysisDimension::RepetitivePhrases,
        score: None,
        data: serde_json::json!({ "top_words": top_100, "total_unique": freq.len() }),
        concerns: vec![],
        computed_at: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_frequency_basic() {
        let doc = DocumentData::test_fixture("hardwired_sample");
        let result = word_frequency(doc);
        assert!(result.data["top_words"].as_array().unwrap().len() > 0);
    }
}
```

### 6.2 LLM 任务实现模板

> 未实现，也没有计划中的实现位置：当前所有维度（13 个度量 + 规则评估 + 由它们派生的审查清单）全部是确定性规则计算
> （`run_xxx_analysis(&doc)`），不存在 LLM 通道。若未来引入 T2 层，必须先
> 解决"同一输入两次结果不同"与缓存键的关系，再考虑落点。

### 6.3 存储层实现模板

```rust
// studio-storage/src/document_repo.rs —— 实际形态：单事务 + Appender 批量写，
// 实体身份在打开 appender 之前经 entity_registry 解析（appender 可变借用 tx）。
pub fn save_document(
    conn: &Connection,
    project_id: Uuid,
    doc: &DocumentData,
    on_progress: Option<&dyn Fn(u64, u64)>,
) -> Result<()> {
    let tx = Transaction::new_unchecked(conn)?;
    // 1. documents 行；2. 为每个 (text, category) 解析 stable_id；
    // 3. blocks/tokens/entities/signals 四个 appender 循环写入
    //    （entities 的第 13 列即 stable_id）；4. flush 检查错误后 commit。
    // 任一失败整体回滚，不留半截文档。
    // ...
    Ok(())
}

// studio-storage/src/entity_registry.rs —— 稳定身份：
// stable_id = "ent:" + sha256(category ⿟ normalize(text)) 前 16 hex。
// 同一身份跨导入、跨项目得到同一 id；v13 之前的 NULL 行按同一公式在读取侧回填。
```

### 6.4 日志规范

```rust
use tracing::{info, instrument};

#[instrument(skip(self), fields(project_id = %project_id))]
pub async fn run_analysis(&self, project_id: &str) -> Result<()> {
    info!("Starting analysis pipeline");

    let t0_start = std::time::Instant::now();
    self.run_t0_tasks().await?;
    info!(elapsed_ms = t0_start.elapsed().as_millis() as u64, "T0 tasks complete");

    let t1_start = std::time::Instant::now();
    self.run_t1_tasks().await?;
    info!(elapsed_ms = t1_start.elapsed().as_millis() as u64, "T1 tasks complete");

    Ok(())
}
```

### 6.5 分析结果缓存

分析是纯函数（同一文档 → 同一结果），所以结果按输入指纹持久化，二次打开项目直接读缓存而不重算。

**缓存键**：`(project_id, dimension, input_hash)`，其中 `dimension ∈ {t0, t1, plot, pacing, conflict, theme, setting, cliche, dialogue, explicit, structure, timeline, voice, checklist, assessment}`。情节线在 `score` 列存 `coherence_score`、节奏在 `score` 列存 `pace_score`、冲突在 `score` 列存 `conflict_score`、主题在 `score` 列存 `theme_score`、场景在 `score` 列存 `setting_score`、陈词滥调在 `score` 列存 `cliche_score`（未检测到冲突用词、没有可用名词词汇、既没有点名地点也没有环境用词、或全文不足 1000 字不足以按万字密度评分时该列为 NULL——缺席本身也要被记住，读回来仍然是「未评分」而不是 0）；对话/叙事比例、显式语言、故事结构、时间线、作者声音与最终审查清单则**永远没有分数**：比例不是评级，30% 对白并不比 5% 更好或更坏；内容也不是评级，一部战争题材并不比田园题材更差；形状更不是评级，契合三幕不是优点，有意拒绝这个形状也不是缺点；时间也不是评级，一部从不写日期的故事并不比每页挂一份日历的故事差；写法同样不是评级，长句不比短句好，留余地的叙述也不比下断言的叙述差；清单更不是评级，每一项是一个待回答的问题，而问题没有分数。所以这六个维度的 `score` 恒为 NULL 而不是 0。这样分析历史不必解开 payload 也能横向比较。新增维度不需要改 `ANALYSIS_VERSION`：旧库缺少该维度的行，读取时天然 miss，会重算并补齐全部维度。清单是**派生行**（由其余维度的结果算出，不读正文），改它的规则或阈值要 bump `ANALYSIS_VERSION`，否则同一份缓存里会出现新清单配旧数字（见 §6.9）。

`input_hash = SHA-256(ANALYSIS_VERSION ‖ 每个 block 的字节长度 ‖ block 的 JSON)`（`studio-analysis/src/hash.rs`）：

- 走 block 的 JSON 形式：分析器读什么就哈希什么，文档模型新增字段自动被覆盖；
- 混入字节长度：同样的文本、不同的切块方式不会撞哈希；
- **不含 `doc_id`**：每次导入都会生成新的 UUID，纳入哈希会导致重复导入同一份内容时必然失效；
- `ANALYSIS_VERSION` 是盐：分析代码变了而输入没变时，改这个常量即可让全部旧结果失效。

**失效路径**：文本变化 → 哈希变化 → 天然 miss，不需要显式清理；「重新分析」按钮走 `force: true`，先 `invalidate_project` 删除该项目的 `analysis_results` 与 `concerns` 再重算，用于哈希无法感知的情况（分析代码变更）或用户主动要求。

**写入约定**：`save_result` 先按缓存键删除旧行再插入（配合 v11 的唯一索引 `(project_id, dimension, input_hash)`），保证重复分析不堆积历史行、`concerns` 不重复挂接。评估的 `recommendations` 会落为 `concerns` 行，报告导出可直接查询。缓存**写入**失败只记日志、不报错——用户已经拿到结果，代价只是下次重算。

**DuckDB 注意**：`ALTER TABLE ... ADD COLUMN` 不支持带约束的列（`NOT NULL DEFAULT` 会报 "Adding columns with constraints not yet supported"），所以 v11 新增的 `tier` / `input_hash` 是可空列，由读取侧对历史行兜底默认值。

### 6.6 报告导出

`studio_report::render_markdown(&ReportInput { .. })` 是纯函数：同一份分析结果 + 同一个时间戳 → 逐字节相同的 Markdown。时间戳是入参而不是内部读时钟，这样才能用 golden 测试钉死输出（见 `markdown.rs` 的 `test_golden_minimal_report`）。

- **生成**：`report_generate` 复用 `analyze_project_inner`（缓存命中就不重算），渲染后写入 `reports` 表（v12），返回 `ReportMeta`（含 markdown 正文，前端据此建议文件名）。
- **导出**：`report_export` 读取**已入库**的 markdown 写盘并记录 `exported_path`。分成两步是为了「导出的是已入库的那一份」：之后重新分析再导出，不会悄悄换一套数字。
- **写文件在 Rust 侧**完成，前端只用 dialog `save()` 取路径，因此 webview 不需要 filesystem 权限。
- **长列表截断**：词表 15 项、其余列表 20 项，并写明「另有 N 项未列出」；表格单元格转义 `|` 与换行，避免被分析的文本破坏表格。
- **最后一节是最终审查清单**：每项渲染成 `- [ ]`（Markdown 原生复选框）加一行触发它的数字，四组恒全列出。它不新增措辞来源，
  因此 HTML/PDF 无需第二套渲染器就带上这一节；知识球导出是图而不是报告章节，作者声音与清单都不在其中（见 §6.9）。
- **PDF 走 HTML**：`studio_report::render_html(&markdown)` 是纯函数，把**已入库的那份 Markdown**
  转成自包含的打印就绪 HTML（`@page` A4、CJK 字体栈、跨页重复表头、表格行不跨页，屏幕上提示按 ⌘P）。
  它**不是第二套渲染器**：报告的措辞只活在 `markdown.rs` 一处，HTML 只是那份文本的另一种排版——
  两处渲染必然漂移，第二列存储必然腐坏。`report_export_html` 在导出时现渲染，所以打印出来的数字
  仍是生成那一份，之后重新分析也换不掉。真正一键产出 `.pdf` 需要内嵌 CJK 字体的排版引擎，留给 M4。

### 6.7 词库与外部数据出处

时间线维度依赖一批历法数据，都在 `studio-analysis/src/calendar.rs` 里以 `const` 表的形式随代码版本走（不进数据库：它是分析器的一部分，改了就该让旧缓存失效）：

| 表 | 内容 | 出处与许可 |
|------|------|-----------|
| `ERAS` | 年号 → 起止 Gregorian 年（约 1000 行、700 余个别名，新莽至民国凡有年代的年号全部收录） | 公开的历代年号对照表（CC BY-SA 4.0，据李崇智《中国历代年号考》），见仓库根 `NOTICE` |
| `SOLAR_TERMS` / `FESTIVALS` / `HOUR_PERIODS` / `WEEKDAYS` | 24 节气、节日、十二时辰与更次、星期 | 常识词表，手写 |
| 农历↔公历换算、农历年的干支 | `lunar_to_solar_date`、`lunar_ganzhi_index_of_year` | crate `lunar-lite`（MIT，内核来自 `6tail/tyme4rs`，MIT）；走天文算法，不含硬编码年份表 |

扩充时守四条：

- **年号只在带位年数时读作日期**：`正统十四年` 是日期，`正统的观点` 不是。禁止把裸年号名当标记。
- **同名年号用候选集，不用首命中**：`era_candidates` 返回所有对得上的年份，多于一个时标为 partial + `ambiguous_era`（`至元三年` 可能是 1266 或 1337），绝不静默取第一个。
- **一个年号被几个政权沿用，只算一段**：起始年相同、或被另一段整个包含的（`长兴` 在 后唐 到 934、在 吴越 到 933、在 荆南 到 932）合成一段，免得中途沿用它被当成第二个元年；只是相接的（后晋 `天福` 936-944 与后汉 `天福` 944-945）仍是两段。`era_candidates` 再把落在同一年份的重复候选去掉。
- **词表里的字也可能不是时间**：`周` 既是星期也是朝代——`西周末期` 不是周末，前一字是朝代限定词（东/西/南/北/前/后/战）时星期词一律拒读；`清明` 可能是《清明上河图》，`周三多` 可能是作者，这类分不出来。凡是只能按字面识别的（节气 / 节日 / 星期），报告必须把它们数出来并说明（`term_collision`），不能装作确定。
- **新词表不得与既有词表重叠**：一个词落在两张表里，取哪个精度就取决于调用顺序。`calendar::tests::the_term_tables_do_not_overlap_each_other` 钉住这一点；确实同名的（`上元` 既是年号也是节日）靠「有没有位年数」分开，并有测试。
- **改词表或匹配器就 bump `ANALYSIS_VERSION`**（`studio-analysis/src/hash.rs`）：词库变了而文档没变，旧缓存必须失效。报告里的「分析版本」是渲染出来的，改版本要一并改 `studio-report` 的 golden 测试。

`ERAS` 的第三列是**该年号停用的年份**：多数情况下就是最后一个位年数所在年，但新号在年初几个月内改元时会多出一位（`景云` 没有 `景云三年`，仍记到 712）。这一位最坏只影响年号末尾的一个位年数，不会给出错的年份，所以保留来源写的日期，不再手工维护第二列。表是从来源的「起讫时间」列转录的：只收有公元年代的条目（标 `？` 的与公元前条目不收），来源里的繁体写法已转成简体。

第三方数据/代码的许可义务记在仓库根 `NOTICE`（Apache-2.0 §4(d) 的惯例）：年号表那份 CC BY-SA 4.0 的署名与 ShareAlike 提示不能丢，再分发时要一并带上。

### 6.8 作者声音的度量口径

`studio-analysis/src/voice.rs` 量的是一篇文章**怎么写**，与题材无关，因此它没有评分函数：文件里既没有 `*_score`，
`VoiceStats` 也没有该字段，缓存行的 `score` 恒为 NULL。加维度时不要顺手补一个分数。

三条口径决定这些数字能不能横向比：

- **一切密度按可见字符**（`text::visible_char_count`）算，每千字或每万字。分母是同一份文本，两个项目才可直接对照；
  不要用 token 数——token 来自 NLP 管线，切分方式变了密度就变，而这一维度不该依赖 NLP。
- **句与段来自既有切分**：句子用 `text::split_sentences`，段落就是 SemanticBlock（TraceView 的分段），曲线用
  `segment::plan_segments`。这三处任何一处改了，节奏、对白、场景等维度会一起动，所以宁可在这里派生，也不要另写一份切句器。
- **词表按固定顺序扫描**（`MARKS` / `FUNCTION_GROUPS` / `HEDGING` / `ASSERTIVE` / 三组人称），输出顺序即声明顺序，
  不读 HashMap，同一输入必须每次给出同一份 JSON。

按单字计数是**有意为之**，不是偷懒：`的地得`、`了着过`、`之其者乃则亦乎` 与 `我你他` 都是单字功能词，走 token 会把
`了解` 的 `了` 归进动词、`其他` 的 `他` 归进代词，两种错法都比多计一次更难看。人称一侧补了一条最小的排除——`他/她/它`
紧跟 `其/别/单` 时不算人称（`其他`、`别的`、`单它`），由 `test_a_pronoun_inside_a_word_is_not_a_person` 钉住。新增排除词时
保持只往前看一个字：窗口再宽就会开始猜词，而这个维度不做分词。

阈值集中在文件顶部（`LONG_SENTENCE_AVG`、`VARIED_CV`、`COMMA_RUN`、`SNIPPET_PARAGRAPH_SHARE`、`STANCE_RATE` …），
每个都带一句「为什么这个数值得命名」。它们不是测量而是**值得说一说的界线**，所以每条 finding 的 detail 必须把实际数字写进去，
读者不同意界线时能自己看数。`MIN_CHARS_FOR_FINDINGS = 600` 之下只报 `too_short` 并立刻返回：一页纸可以有任何风格。

改词表或阈值要 bump `ANALYSIS_VERSION`（见 §6.7 最后一条）：文档没变而口径变了，旧缓存必须失效。

### 6.9 最终审查清单的口径

`studio-analysis/src/checklist.rs` 不测量任何东西：它的输入是其余维度的结果（`ChecklistSource` 只借引用，不读正文），
输出是「这些数字还需要人决定什么」。因此它没有评分函数，`ReviewChecklist` 里没有任何 `score` 字段，缓存行的 `score`
恒为 NULL——一个问题是问题，不是分数。`severity` 只决定同一组内谁排在前面，UI 与报告都不把它画成严重度。

四条口径决定这份清单能不能被信任：

- **没有数字就不出现**：每条规则都从某个维度已有的计数派生（未收束的线索、日历倒回、无锚点场景、张力持平的长段、
  连续无冲突段、陈词滥调命中、引号未配对……），detail 必须把那个数字写全。规则不新算任何东西，也不从别的规则再推一层。
- **空清单要说明它不能证明什么**：`items` 为空时报告与 UI 都写「只说明被计到的东西不需要决定，不说明稿子已经好」。
  清单永远不可能完整——没被计到的东西不会出现在这里，这句话因此不能省。
- **组是固定的**（`CHECKLIST_GROUPS`：consistency / structure / language / advisory），四组恒全列出，空组写「这一组没有
  需要确认的项」。UI 一组一面板，所以一个组悄悄消失会是 bug 而不是干净的结果。
- **`id` 是勾选的存储键**：改文案可以，改 `id` 会把读者已打的勾落到别的行上。新增项只在末尾追加，顺序即检查顺序。

勾选状态是**阅读进度**而不是分析数据，存在浏览器 `localStorage`（`lib/checklist.ts`，键
`narrativestudio.checklist.{project_id}`，只存 `true`），与主题选择同一处；数据库里没有它，重分析也不会动它。
导出侧不需要新渲染器：Markdown 用 `- [ ]`，HTML/PDF 是那份 Markdown 的另一种排版，知识球导出是另一种产品（图，
不是报告章节），因此作者声音与清单都不在其中。

改规则里的阈值（`UNANCHORED_SHARE`、`ABSENT_RUN_SEGMENTS`、`ADVERB_OVERUSE`、`READABILITY_FLOOR`）与加/删规则都要
bump `ANALYSIS_VERSION`：清单是派生行，但它跟着源维度一起失效才不会与同一份缓存里的其他数字说不同的话。

---

## 7. 测试策略

### 7.1 测试金字塔

| 层级 | 工具 | 覆盖目标 | 位置 |
|------|------|---------|------|
| 单元测试 | `cargo test` | 每个函数/方法 | 同文件 `#[cfg(test)]` |
| 集成测试 | `cargo test` (tests/) | crate 间交互 | `crates/*/tests/` |
| 端到端测试 | Playwright (Tauri) | 用户流程 | `crates/studio-app/e2e/` |
| 前端单元测试 | Vitest | 组件/工具函数 | `crates/studio-app/src/**/*.test.ts` |

### 7.2 测试数据

集成测试使用**脱敏后的 TraceView 子集**作为 fixture，已提交进仓库，因此
`cargo test --workspace` 在任意机器与 CI 上都能运行（不再依赖本机 `~/.TraceView/`）：

```
crates/studio-app/src-tauri/tests/fixtures/
├── traceview_semantic_sample.json   # 12 blocks / 886 tokens / 108 entities
└── README.md                        # 来源、脱敏方式与再生成方法
```

脱敏采用**等长 CJK 字符一对一替换**：`span` 偏移、token 数、实体数、实体类别、
POS 与置信度全部与原文档一致，只有文字内容失去语义。生成脚本：

```bash
python3 scripts/gen_fixture.py            # 自动挑选信息量最大的 12 块窗口
```

`NARRATIVE_TEST_FILE` 环境变量可覆盖 fixture 路径，用于对完整真实数据做冒烟。

后续新增 fixture（popo / enriched / expected 快照）时，放在同一 `fixtures/`
目录下，并在 README 中登记来源与脱敏方式。

### 7.3 测试命令

```bash
cargo test --workspace                    # 全部
cargo test -p studio-analysis             # 单 crate
cargo test -p studio-analysis tasks::t0_stats  # 单模块
cargo test -p studio-app --test integration    # 集成测试（读仓库内 fixture）

pnpm test                                 # 前端单测（仓库根，转发到 studio-app）
pnpm lint                                 # 前端 lint
pnpm typecheck                            # 前端类型检查
cd crates/studio-app && pnpm test:watch   # 前端 watch 模式

cargo llvm-cov --workspace                # 覆盖率（尚未纳入 CI 门禁）
```

### 7.4 真实数据冒烟（`#[ignore]` 测试）

仓库内的 fixture 只有 12 块，证明不了规模，也证明不了误报。`crates/studio-app/src-tauri/tests/timeline_scale.rs`
用本机真实 TraceView 导出跑时间线，**不进 CI**：

```bash
# 单本：耗时 + 全部历法读数（年号/干支/农历/节气/节日/星期）连上下文
NARRATIVE_TEST_FILE=~/.TraceView/<书>/semantic/semantic_result.json \
  cargo test -p studio-app --test timeline_scale -- --ignored --nocapture

# 整个目录：逐本跑一遍，把每一处历法读数打出来供人眼判读
NARRATIVE_TEST_DIR=~/.TraceView \
  cargo test -p studio-app --test timeline_scale -- --ignored --nocapture
```

两者都断言 T1 预算（单遍分析 < 2s；JSON 解析归导入，不算在这里），并把来自词表的
读数连上下文一并打印——**误报只能靠人眼看**，所以上下文是测试输出的一部分。

实测（本机 26 份可读导出 · 814 万字，最慢一本 174 万字 · 7258 块）：

| 口径 | 最慢一本 | 结论 |
|------|---------|------|
| dev profile（`cargo test`） | 1.1–1.4 s | 在 2s 预算内，但只剩 1.4× 余量 |
| release profile | 88–155 ms | 比 dev 快约 10×，发布口径下余量 13× 以上 |

真实数据找到的两处问题都已进单元测试：块文本以数量词结尾时 `match_buckets` 越界
（现在边界安全），以及 `西周末期` 被 `周末` 命中（见 6.7 的朝代规则）。

### 7.5 CI 测试门禁

下表已在 `.github/workflows/ci.yml` 中生效：

| 检查 | 通过标准 | 状态 |
|------|---------|------|
| `cargo fmt --all --check` | 无差异 | ✅ 已启用 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 零警告 | ✅ 已启用 |
| `cargo test --workspace` | 全部通过 | ✅ 已启用 |
| `pnpm typecheck` | 零错误 | ✅ 已启用 |
| `pnpm lint` | 零错误 | ✅ 已启用 |
| `pnpm test` | 全部通过 | ✅ 已启用 |
| `pnpm build` | 构建成功 | ✅ 已启用 |
| 覆盖率 studio-core ≥ 90% / studio-analysis ≥ 80% | — | ⏳ 待接入 `cargo llvm-cov` |

CI 与本地编译使用**同一个**编译器：`rust-toolchain.toml` 钉住版本（当前 1.93.0），
CI 通过 `rustup show` 安装它。原因是 clippy 会在小版本之间把 lint 从 nursery 提升
为默认警告——本地工具链落后于 CI 的 rolling `stable` 时，会出现「本地全绿、CI 报
新 lint」。升级方式：本地 `rustup update` → 修掉新版 clippy 报的问题 → 在同一个
commit 里更新 `rust-toolchain.toml`。

---

## 8. 构建与发布

### 8.1 本地构建

```bash
pnpm --filter studio-app tauri build --debug   # 开发
pnpm --filter studio-app tauri build           # 发布
```

### 8.2 CI/CD (GitHub Actions)

实际配置见 `.github/workflows/ci.yml`（另有 `pr-rule-check.yml`、
`issue-template-check.yml` 做模板与规则校验）。三个 job：

| Job | 触发 | 内容 |
|-----|------|------|
| `rust` | push / PR | 安装 Tauri Linux 系统库与 C++ 工具链 → `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace` |
| `frontend` | push / PR | `pnpm install --frozen-lockfile` → `typecheck` → `lint` → `test` → `build`（工作目录 `crates/studio-app`） |
| `package` | 仅 `workflow_dispatch` | macOS 上 `pnpm --filter studio-app exec tauri build` |

两处与早期设计稿不同的地方，是实测后的有意取舍：

1. **rust job 必须装系统依赖**。`studio-app` 是 workspace 成员，`cargo test
   --workspace` 会编译它，缺 `libwebkit2gtk-4.1-dev` 等库直接失败；DuckDB 走
   `bundled` 特性从源码编译，还需要 `clang` / `cmake`。
2. **打包不进 PR 门禁**。release 模式编译 DuckDB 约 20 分钟，而 rust job 已经
   证明过同样的代码能编译与通过测试，因此打包改为手动触发，发版前执行。

`Cargo.lock` 已纳入版本控制（应用项目而非库），CI 与发布构建因此使用固定的依赖版本。

### 8.3 版本管理

遵循 Semantic Versioning。发布流程：更新版本号 → 更新 CHANGELOG → 打 tag → CI 自动构建发布。

---

## 9. 调试指南

### 9.1 后端调试

```bash
# 带日志运行
RUST_LOG=studio_analysis=debug,studio_storage=trace pnpm --filter studio-app tauri dev

# 仅运行后端（无 GUI）
cargo run -p studio-app -- --headless --test-project /path/to/traceview

# 性能分析
cargo flamegraph -p studio-analysis -- test_word_frequency
```

### 9.2 前端调试

```bash
# 浏览器中开发（mock IPC）
cd crates/studio-app && pnpm dev  # http://localhost:5173
# 在 src/lib/mock.ts 中实现 mock invoke/listen
```

### 9.3 常见问题

| 问题 | 原因 | 解决 |
|------|------|------|
| Tauri dev 白屏 | 前端 dev server 未启动 | 确认 `pnpm dev` 在 `crates/studio-app` 运行 |
| IPC 调用超时 | 后端任务阻塞 | 检查是否误用同步 I/O，改用 `tokio::fs` |
| DuckDB 锁冲突 | 多进程访问同一 DB | 确保单实例运行，或改用 WAL 模式 |
| LLM 超时 | —（当前无 LLM 调用） | 不适用 |
| 内存泄漏 | 大 JSON 未释放 | 分析完成后显式 `drop()` 中间数据 |

### 9.4 数据目录结构

当前实现：数据库在 `app_data_dir`（macOS 下 `~/Library/Application Support/`）里的
`narrative_studio.duckdb`，正文与实体全部在库内；`projects.source_path` 原位指向 TraceView
项目文件夹，**不复制**源文件。下面的 `projects/{project_id}/` 与 `logs/` 是目标形态（见
product-design §10.2 的未完成项），还没有代码创建它们。

```
~/.NarrativeStudio/
├── narrative_studio.duckdb    # 主数据库
├── projects/
│   └── {project_id}/
│       ├── project.json       # 项目元数据
│       ├── source/            # 原始 TraceView 文件（只读副本）
│       │   ├── semantic_result.json
│       │   ├── popo_result.json
│       │   └── enriched_result.json
│       └── reports/           # 导出的报告
└── logs/
    └── studio.log             # 应用日志
```

---

## 10. 贡献指南

### 10.1 开发流程

```bash
git checkout -b feat/analysis-word-frequency
cargo test -p studio-analysis
pnpm --filter studio-app tauri dev  # 手动验证
git commit -m "feat(analysis): add word frequency T0 task"
git push origin feat/analysis-word-frequency
```

### 10.2 PR 要求

- [ ] 所有 CI 检查通过（clippy, fmt, test, lint）
- [ ] 新增功能有对应测试
- [ ] 公共 API 变更更新文档
- [ ] 无 `unwrap()`/`expect()` 在生产代码中
- [ ] 至少 1 个 reviewer 批准

### 10.3 分支策略

| 分支 | 用途 |
|------|------|
| `main` | 稳定版本，保护分支 |
| `dev` | 开发集成 |
| `feat/*` | 新功能 |
| `fix/*` | 修复 |
| `chore/*` | 构建/工具 |

### 10.4 代码审查关注点

| 维度 | 检查项 |
|------|--------|
| 正确性 | 逻辑是否正确，边界条件是否处理 |
| 性能 | 是否有不必要的 O(n²)，大数据是否流式处理 |
| 安全 | 是否有路径遍历、SQL 注入风险 |
| 可维护性 | 命名是否清晰，是否遵循现有模式 |
| 测试 | 是否覆盖关键路径，测试是否有意义 |
| 文档 | 公共 API 是否有文档注释 |

---

## 附录 A：快速参考卡片

```
┌─────────────────────────────────────────────────────────────┐
│  NarrativeStudio 开发快速参考                                 │
├─────────────────────────────────────────────────────────────┤
│  开发运行:   pnpm --filter studio-app tauri dev             │
│  后端测试:   cargo test --workspace                         │
│  前端测试:   cd crates/studio-app && pnpm test              │
│  格式化:     cargo fmt --workspace && pnpm format           │
│  Lint:       cargo clippy --workspace -- -D warnings        │
│  构建:       pnpm --filter studio-app tauri build           │
│  日志:       RUST_LOG=debug pnpm --filter studio-app tauri dev │
│  数据目录:   ~/.NarrativeStudio/                            │
├─────────────────────────────────────────────────────────────┤
│  Crate 依赖方向:                                             │
│  studio-app → studio-{core,import,analysis,storage,report}  │
│  studio-{import,analysis,storage,report} → studio-core      │
│  studio-core → (无内部依赖)                                  │
├─────────────────────────────────────────────────────────────┤
│  任务层级:                                                   │
│  T0 (纯统计) → 并行, <100ms, 无 LLM                         │
│  T1 (结构)   → 并行, <2s, 无 LLM                            │
│  T2 (LLM)   → 串行, 5-60s, 可选                             │
└─────────────────────────────────────────────────────────────┘
```