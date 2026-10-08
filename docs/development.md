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
pnpm add zustand
pnpm add d3
pnpm add -D tailwindcss postcss autoprefixer
npx tailwindcss init

# 目录结构
mkdir -p src/{components,pages,stores,lib,types}
```

### 1.3 目录结构总览

```
NarrativeStudio/
├── Cargo.toml                      # Workspace root
├── crates/
│   ├── studio-core/                # 核心类型（无业务逻辑）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── project.rs
│   │       ├── document.rs
│   │       ├── entity.rs
│   │       ├── noun_signal.rs
│   │       ├── analysis.rs
│   │       ├── concern.rs
│   │       └── report.rs
│   │
│   ├── studio-import/              # 数据导入
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traceview.rs
│   │       ├── project_store.rs
│   │       └── validator.rs
│   │
│   ├── studio-analysis/            # 分析引擎
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── orchestrator.rs
│   │       ├── tasks/
│   │       │   ├── mod.rs
│   │       │   ├── t0_stats.rs
│   │       │   ├── t1_structural.rs
│   │       │   └── t2_llm.rs
│   │       ├── llm/
│   │       │   ├── mod.rs
│   │       │   ├── client.rs
│   │       │   ├── prompts.rs
│   │       │   └── parser.rs
│   │       ├── cache.rs
│   │       └── types.rs
│   │
│   ├── studio-storage/             # DuckDB 持久化
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs
│   │       ├── migrations.rs
│   │       └── repos/
│   │           ├── mod.rs
│   │           ├── project_repo.rs
│   │           ├── entity_repo.rs
│   │           └── analysis_repo.rs
│   │
│   ├── studio-report/              # 报告生成
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── aggregator.rs
│   │       ├── markdown.rs
│   │       └── json.rs
│   │
│   └── studio-app/                 # Tauri 应用
│       ├── package.json
│       ├── src/                    # 前端 (React)
│       │   ├── main.tsx
│       │   ├── App.tsx
│       │   ├── components/
│       │   ├── pages/
│       │   ├── stores/
│       │   ├── lib/
│       │   └── types/
│       └── src-tauri/              # Tauri 后端入口
│           ├── Cargo.toml
│           ├── tauri.conf.json
│           └── src/
│               ├── main.rs
│               ├── lib.rs
│               └── commands/
│                   ├── mod.rs
│                   ├── project.rs
│                   ├── import.rs
│                   ├── analyze.rs
│                   ├── report.rs
│                   └── settings.rs
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
| `STUDIO_LLM_PROVIDER` | LLM 提供商 | `none` |
| `STUDIO_LLM_API_KEY` | LLM API Key | - |
| `STUDIO_LLM_MODEL` | LLM 模型名 | - |
| `STUDIO_LLM_BASE_URL` | LLM 自定义 URL（Ollama） | - |
| `RUST_LOG` | 日志级别 | `info` |

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
| 状态 | Zustand store，按领域拆分（`useProjectStore`, `useAnalysisStore`） |
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

所有 Tauri Commands 定义在 `studio-app/src-tauri/src/commands/` 中，前端通过 `@tauri-apps/api/tauri` 的 `invoke` 调用。

| Command | 参数 | 返回 | 说明 |
|---------|------|------|------|
| `project_create` | `{ name, description?, genre? }` | `Project` | 创建项目 |
| `project_list` | - | `Vec<ProjectSummary>` | 列出所有项目 |
| `project_get` | `{ project_id }` | `Project` | 获取项目详情 |
| `project_delete` | `{ project_id }` | - | 删除项目 |
| `import_traceview` | `{ project_id, path }` | `ImportResult` | 导入 TraceView 成果 |
| `import_document` | `{ project_id, path, format }` | `ImportResult` | 导入原始文档 |
| `analyze_project` | `{ project_id, options }` | `TaskId` | 启动分析（后台） |
| `analyze_cancel` | `{ task_id }` | - | 取消分析 |
| `analyze_status` | `{ task_id }` | `TaskStatus` | 查询任务状态 |
| `report_generate` | `{ project_id, format }` | `ReportMeta` | 生成报告 |
| `report_export` | `{ project_id, format, path }` | `ExportResult` | 导出报告 |
| `settings_get` | - | `Settings` | 获取设置 |
| `settings_update` | `{ settings }` | - | 更新设置 |

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
// studio-app/src-tauri/src/commands/analyze.rs
use crate::state::AppState;
use studio_core::analysis::AnalysisOptions;
use tauri::State;

#[tauri::command]
pub async fn analyze_project(
    state: State<AppState>,
    project_id: String,
    options: AnalysisOptions,
) -> Result<String, String> {
    let task_id = state
        .orchestrator
        .start_analysis(&project_id, options)
        .map_err(|e| e.to_string())?;
    Ok(task_id)
}

#[tauri::command]
pub async fn analyze_cancel(
    state: State<AppState>,
    task_id: String,
) -> Result<(), String> {
    state
        .orchestrator
        .cancel(&task_id)
        .map_err(|e| e.to_string())
}
```

---

## 5. 前端开发指南

### 5.1 页面结构

| 页面 | 路由 | 说明 |
|------|------|------|
| 项目列表 | `/` | 所有项目卡片 |
| 项目详情 | `/project/:id` | 单项目分析视图 |
| 多项目对比 | `/compare` | 选择多个项目对比 |
| 设置 | `/settings` | LLM 配置、数据目录 |

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
│       └── SettingsPage
└── Toast / Modal
```

### 5.3 状态管理

```typescript
// stores/useAnalysisStore.ts
import { create } from 'zustand';
import { invoke, listen } from '@tauri-apps/api/event';

interface AnalysisState {
  status: AnalysisStatus;
  progress: { current: number; total: number; label: string };
  results: Record<string, DimensionResult>;
  errors: Record<string, string>;

  startAnalysis: (projectId: string, options: AnalysisOptions) => Promise<void>;
  cancelAnalysis: (taskId: string) => Promise<void>;
  reset: () => void;
}

export const useAnalysisStore = create<AnalysisState>((set) => ({
  status: 'idle',
  progress: { current: 0, total: 0, label: '' },
  results: {},
  errors: {},

  startAnalysis: async (projectId, options) => {
    set({ status: 'loading', results: {}, errors: {} });
    const taskId = await invoke('analyze_project', { projectId, options });

    await listen('analysis:progress', (e) => {
      set({ progress: e.payload });
    });
    await listen('analysis:result', (e) => {
      const { dimension, data } = e.payload;
      set((s) => ({
        results: { ...s.results, [dimension]: data },
        status: 'partial',
      }));
    });
    await listen('analysis:complete', () => {
      set({ status: 'complete' });
    });
  },

  cancelAnalysis: async (taskId) => {
    await invoke('analyze_cancel', { taskId });
    set({ status: 'idle' });
  },

  reset: () => set({ status: 'idle', results: {}, errors: {} }),
}));
```

### 5.4 图表组件规范

- 所有图表使用 **SVG 渲染**（非 Canvas），支持响应式
- 使用 D3.js 做数据绑定和比例尺，手动管理 SVG 元素
- 每个图表组件接收 `data` + `width` + `height` props
- 颜色方案：项目 A=#3B82F6(蓝), B=#F97316(橙), C=#22C55E(绿), D=#A855F7(紫), E=#EF4444(红)
- 暗色模式：通过 CSS 变量切换，图表颜色适配

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

```rust
// studio-analysis/src/tasks/t2_llm.rs
use crate::llm::{LlmClient, LlmRequest};
use crate::types::TaskContext;
use studio_core::analysis::{AnalysisDimension, AnalysisResult};
use studio_core::error::{Result, StudioError};

/// T2: 叙事弧线识别
pub async fn narrative_arc(
    llm: &LlmClient,
    ctx: &TaskContext,
) -> Result<AnalysisResult> {
    // 1. 构建 prompt
    let prompt = build_arc_prompt(&ctx.chapter_summaries, &ctx.structure_tree);

    // 2. 调用 LLM
    let response = llm
        .complete(&LlmRequest {
            model: llm.model.clone(),
            messages: vec![
                ("system", "You are a narrative analysis expert...").into(),
                ("user", &prompt).into(),
            ],
            max_tokens: 2000,
            temperature: 0.3,
        })
        .await?;

    // 3. 解析输出
    let parsed = parse_arc_response(&response.text)?;

    Ok(AnalysisResult {
        dimension: AnalysisDimension::NarrativeArc,
        score: Some(parsed.overall_score),
        data: serde_json::to_value(parsed)?,
        concerns: parsed.concerns,
        computed_at: chrono::Utc::now(),
    })
}
```

### 6.3 存储层实现模板

```rust
// studio-storage/src/repos/entity_repo.rs
use duckdb::params;
use studio_core::entity::{Entity, EntityCategory};
use studio_core::error::Result;

pub struct EntityRepo<'conn> {
    conn: &'conn duckdb::Connection,
}

impl<'conn> EntityRepo<'conn> {
    pub fn insert_batch(&self, doc_id: &str, entities: &[Entity]) -> Result<usize> {
        let mut count = 0;
        for e in entities {
            self.conn.execute(
                "INSERT INTO entities (entity_id, block_id, doc_id, text, category, confidence, source, keep, span_start, span_end)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    e.block_id, doc_id, e.text,
                    e.category.as_str(), e.confidence,
                    e.source, e.keep,
                    e.span.0 as i64, e.span.1 as i64,
                ],
            )?;
            count += 1;
        }
        Ok(count)
    }

    pub fn query_by_category(&self, doc_id: &str, category: &str) -> Result<Vec<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT text, category, confidence, source, keep, span_start, span_end
             FROM entities WHERE doc_id = ?1 AND category = ?2 AND keep = TRUE"
        )?;
        let rows = stmt.query_map(params![doc_id, category], |row| {
            Ok(Entity {
                text: row.get(0)?,
                category: EntityCategory::from_str(row.get(1)?),
                confidence: row.get(2)?,
                source: row.get(3)?,
                keep: row.get(4)?,
                filter: None,
                filter_reason: None,
                span: (row.get(5)? as usize, row.get(6)? as usize),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
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

    if self.llm_enabled() {
        let t2_start = std::time::Instant::now();
        self.run_t2_tasks().await?;
        info!(elapsed_ms = t2_start.elapsed().as_millis() as u64, "T2 tasks complete");
    }

    Ok(())
}
```

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

### 7.4 CI 测试门禁

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
| LLM 超时 | 网络/API 问题 | 检查 `STUDIO_LLM_BASE_URL`，增加 timeout |
| 内存泄漏 | 大 JSON 未释放 | 分析完成后显式 `drop()` 中间数据 |

### 9.4 数据目录结构

```
~/.NarrativeStudio/
├── studio.duckdb              # 主数据库
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