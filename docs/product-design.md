# NarrativeStudio 产品设计文档

> 版本：v0.1.0（初稿）
> 日期：2026-09-14
> 状态：Draft
> 技术栈：Rust + Tauri 2.x 跨平台桌面应用

---

## 目录

1. [产品定位与背景](#1-产品定位与背景)
2. [竞品分析：My Marlowe Report 截图提取](#2-竞品分析my-marlowe-report-截图提取)
3. [数据源：TraceView 加工成果](#3-数据源traceview-加工成果)
4. [核心功能设计](#4-核心功能设计)
5. [分析维度矩阵](#5-分析维度矩阵)
6. [技术架构](#6-技术架构)
7. [数据模型](#7-数据模型)
8. [UI/UX 设计](#8-uiux-设计)
9. [模块划分与 Crate 结构](#9-模块划分与-crate-结构)
10. [里程碑与路线图](#10-里程碑与路线图)
11. [非功能性需求](#11-非功能性需求)

---

## 1. 产品定位与背景

### 1.1 产品定位

**NarrativeStudio** 是 NarrativeOS 产品矩阵中的**专家级知识工作台**（代号 Kognisynth），以 Rust + Tauri 实现跨平台桌面应用。

核心价值：**将 TraceView 产生的数据加工成果（语义分析、实体提取、名词信号等）进行多维度建模分析，生成结构化的叙事诊断报告。**

### 1.2 目标用户

| 角色 | 场景 |
|------|------|
| 小说作者 / 网络文学创作者 | 初稿诊断、结构优化、出版前审查 |
| 剧本创作者 / 编剧 | 情节分析、角色弧线、节奏把控 |
| 文学编辑 / 内容编辑 | 文本质量评估、问题定位、修改建议 |
| 创意写作教师 / 学生 | 教学分析、写作练习反馈 |
| 领域专家 / 知识工程师 | 非结构化文档知识提取与图谱构建 |

### 1.3 产品边界

```
┌─────────────────────────────────────────────────────────────┐
│  NarrativeStudio (Rust + Tauri)                             │
│                                                             │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │ 数据导入层   │    │ 分析引擎层    │    │ 报告展示层     │  │
│  │             │    │              │    │               │  │
│  │ • TraceView │    │ • 叙事分析    │    │ • 多维度报告   │  │
│  │   结果导入   │───▶│ • 文本诊断    │───▶│ • 可视化图表   │  │
│  │ • 原始文档   │    │ • 统计分析    │    │ • 优先级建议   │  │
│  │   直接导入   │    │ • 内容分析    │    │ • 导出/分享    │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
│                                                             │
│  数据底座：DuckDB（本地） │ 云能力：可选层                     │
└─────────────────────────────────────────────────────────────┘
```

### 1.4 与产品矩阵的关系

```
TraceView (Caly, .NET/Avalonia)
  │  数据加工成果 (semantic_result.json, popo_result.json)
  │  • NER 实体流（PERSON/ORG/LOCATION/DATE/SECTION_REF...）
  │  • 名词信号（noun_signals: POS+句法涌现的领域术语）
  │  • 语义块分析结果
  │  • 文档结构树（TOC、章节路径）
  ▼
NarrativeStudio (Rust + Tauri)  ← 本项目
  │  多维度建模分析
  │  • 叙事弧线 / 情节 / 角色 / 冲突 / 主题
  │  • 文本诊断 / 统计 / 内容分析
  │  • 综合评估与改进建议
  │
  ├──▶ 结构化诊断报告（Markdown，面向人阅读）
  │
  └──▶ .knowsphere.json 知识球文件（机器可读，v1.0 契约见 §4.5）
        ▼
      NarratSphere (Web, 3D 知识球聚合与呈现)
```

---

## 2. 竞品分析：My Marlowe Report 截图提取

### 2.1 截图来源

截图来自 **My Marlowe Report**（Marlowe 平台的 AI 手稿分析报告），展示的是 "Points of concern"（关注点）页面的完整 UI。

### 2.2 左侧导航栏 — 完整分析维度列表

截图左侧导航栏列出了 **24 个分析维度**，按功能分组如下：

| 序号 | 维度名称（英文） | 维度名称（中文） | 分类 |
|------|-----------------|-----------------|------|
| 1 | Get started | 入门引导 | 导航 |
| 2 | Overall assessment | 总体评估 | 评估 |
| 3 | Points of concern | 关注点 / 潜在问题 | 评估 |
| 4 | Narrative arc analysis | 叙事弧线分析 | 叙事分析 |
| 5 | Plot analysis | 情节分析 | 叙事分析 |
| 6 | Story elements | 故事元素 | 叙事分析 |
| 7 | Character analysis | 角色分析 | 叙事分析 |
| 8 | Pacing analysis | 节奏分析 | 叙事分析 |
| 9 | Conflict analysis | 冲突分析 | 叙事分析 |
| 10 | Theme analysis | 主题分析 | 叙事分析 |
| 11 | Setting analysis | 场景/设定分析 | 叙事分析 |
| 12 | Author voice | 作者声音/文风 | 风格分析 |
| 13 | Dialogue/narrative | 对话/叙事比例 | 风格分析 |
| 14 | Readability | 可读性 | 文本诊断 |
| 15 | Clichés finder | 陈词滥调查找器 | 文本诊断 |
| 16 | Explicit language | 显式/敏感语言 | 内容分析 |
| 17 | Repetitive phrases | 重复短语检测 | 文本诊断 |
| 18 | Adverbs/adjectives | 副词/形容词检测 | 文本诊断 |
| 19 | Misspellings finder | 拼写错误查找器 | 文本诊断 |
| 20 | Key recommendations | 关键建议 | 评估 |
| 21 | Story structure | 故事结构 | 叙事分析 |
| 22 | Review & revise | 审查与修订 | 工作流 |
| 23 | Feedback | 反馈 | 工作流 |
| 24 | Next steps | 后续步骤 | 工作流 |

### 2.3 主内容区 — "Points of concern" 页面

#### 2.3.1 Overview 说明

> When you're ready to upload a work, our AI will analyze the story and spotlight areas of the manuscript that could be refined to increase clarity, resolve inconsistencies, and avoid scenes that seem forced or out of place. It will also review the work for any potential issues that might warrant a trigger warning or reader advisory.

**中文翻译：** 当你准备好上传作品时，AI 将分析故事并突出手稿中可改进的区域，以提高清晰度、解决不一致性，并避免显得生硬或不合时宜的场景。它还会审查作品中可能需要触发警告或读者提示的潜在问题。

#### 2.3.2 付费墙

> Subscribe to **Marlowe Pro** to see the points of concern identified in Marlowe's analysis of your manuscript.

**中文翻译：** 订阅 **Marlowe Pro** 以查看 Marlowe 分析中识别的手稿关注点。

（按钮：**GO PRO**）

#### 2.3.3 示例报告（SAMPLE ONLY）

示例报告标题：**Points of concern**
标签：`Debut`（处女作）/ `Experienced`（有经验）

**📌 Possible areas for improvement and potential issues**

> In this section, we'll spotlight areas of your work that could be refined to increase clarity, resolve inconsistencies, and avoid sequences that seem forced or out of place.

**5 条具体问题：**

| # | 问题描述 | 问题类型 |
|---|---------|---------|
| 1 | **Dan's transformation from helpful journalist to killer feels too abrupt** — his obsession with Tobey needs more gradual development throughout the story to make his revelation feel earned rather than shocking purely for plot purposes. | 角色弧线断裂 |
| 2 | **Tobey's complete unawareness of Ty's emotional distance seems unrealistic** — for a man who loves his wife and works closely with her daily, his failure to notice her growing detachment from their marriage strains credibility. | 角色行为不合理 |
| 3 | **The timing of Chase's trade to the Yankees feels too convenient** — while it serves the plot, the confluence of events that brings him back precisely when the team needs him and when Ty is most vulnerable seems orchestrated rather than organic. | 情节巧合/机械降神 |
| 4 | **Ty's professional competence versus personal confusion creates inconsistency** — she demonstrates sophisticated business judgment and team management skills but makes repeatedly poor personal decisions that seem out of character for someone so analytical. | 角色一致性矛盾 |
| 5 | **The serial killer plot connection to the romance feels forced** — while both plots involve themes of obsession and loyalty, the way Dan's murders specifically target Tobey's affairs stretches credibility and makes the thriller elements feel grafted onto the romance rather than naturally integrated. | 情节线融合生硬 |

### 2.4 竞品设计要点提取

从 My Marlowe Report 的设计中，我们提取以下可借鉴的设计模式：

| 设计模式 | 说明 | NarrativeStudio 应用 |
|---------|------|---------------------|
| 左侧导航 + 右侧内容 | 24 个维度按功能分组，当前选中高亮 | 采用相同布局，按 5 大分类分组 |
| 付费墙 + 示例 | 免费看示例，付费看完整报告 | 本地应用无付费墙，全部功能可用 |
| 问题分级 | 每条问题有具体位置 + 严重等级 | 继承，增加可点击跳转到原文位置 |
| 标签系统 | Debut / Experienced 区分读者水平 | 增加：Genre（类型）、Language（语言）标签 |
| 综合评分 | 多维度雷达图 | 继承，增加维度权重可配置 |
| 优先级建议 | 按优先级排序的改进建议 | 继承，增加修改示例和预期效果 |

---

## 3. 数据源：TraceView 加工成果

### 3.1 TraceView 数据输出

TraceView（Caly）作为 PDF 阅读与语义分析客户端，产出以下数据加工成果：

#### 3.1.1 `semantic_result.json` — 语义分析结果

```json
{
  "blocks": [
    {
      "block_id": "blk_001",
      "text": "...",
      "section_path": "第3章 > 3.2 > 3.2.1",
      "unit_type": "text",
      "entities": [
        {
          "text": "张三",
          "category": "PERSON",
          "confidence": 0.92,
          "source": "ner/ontonotes",
          "keep": true,
          "filter": null,
          "filter_reason": null,
          "span": [12, 15]
        }
      ],
      "noun_signals": [
        {
          "text": "绝缘电阻",
          "pos": "NN",
          "syntactic_role": "object_of_predicate",
          "score": 0.72,
          "span": [12, 16],
          "evidence": {
            "head_rel": "dobj",
            "governing_verb": "测量"
          }
        }
      ]
    }
  ],
  "document_signals": {
    "noun_signal_clusters": [],
    "entity_quality": {}
  }
}
```

#### 3.1.2 `popo_result.json` — 文档结构解析

```json
{
  "tree_root": {
    "node_id": "root",
    "type": "document",
    "children": [
      {
        "node_id": "ch1",
        "type": "chapter",
        "title": "第一章",
        "toc_entries": [],
        "children": []
      }
    ]
  },
  "blocks": {
    "blk_001": {
      "type": "text",
      "content": "...",
      "locations": []
    }
  }
}
```

#### 3.1.3 数据加工管线

```
PDF 文档
  │
  ▼
[MinerU OCR] → 文字/表格/图片分离
  │
  ▼
[文档结构解析] → TOC、章节树、SemanticBlock
  │
  ▼
[NLP 语义分析] → tokens + POS + deps + NER
  │
  ├──▶ NER 实体流（减法 F0-F5 清理）
  │     • PERSON / ORG / LOCATION / DATE
  │     • SECTION_REF / NUMBER / STANDARD
  │     • keep/filter/filter_reason 软标记
  │
  ├──▶ 名词信号流（加法 POS+句法涌现）
  │     • 词性门控 (Step A)
  │     • 句法加权 (Step B)
  │     • 全局聚类排序 (Step C, 调用方)
  │
  └──▶ 文档结构树
        • 章节路径 (SectionPath)
        • TOC 标题术语
        • 表格数据
```

### 3.2 NarrativeStudio 数据消费策略

| TraceView 产出 | NarrativeStudio 消费方式 |
|---------------|------------------------|
| NER 实体流 | 角色/地点/时间线提取 → 角色分析、场景分析 |
| 名词信号 | 主题/术语提取 → 主题分析、设定分析 |
| 文档结构树 | 章节/场景划分 → 节奏分析、叙事弧线 |
| 语义块 | 文本诊断基础 → 拼写、陈词滥调、重复检测 |
| 表格数据 | 参数/数据提取 → 设定一致性检查 |

---

## 4. 核心功能设计

### 4.1 功能总览

```
┌─────────────────────────────────────────────────────────────────┐
│                        NarrativeStudio                          │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  数据导入     │  │  分析引擎     │  │  报告与可视化         │  │
│  │              │  │              │  │                      │  │
│  │ • 导入       │  │ • 文本诊断    │  │ • 综合评估报告        │  │
│  │   TraceView  │──▶│ • 统计分析    │──▶│ • 维度雷达图        │  │
│  │   结果       │  │ • 叙事分析    │  │ • 问题清单          │  │
│  │ • 导入       │  │ • 内容分析    │  │ • 优先级建议        │  │
│  │   原始文档   │  │ • 评估建议    │  │ • 可视化图表        │  │
│  │ • 项目       │  │              │  │ • 导出 (MD/PDF/JSON)│  │
│  │   管理       │  │              │  │                      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  数据底座：DuckDB（本地） │ 云能力：可选层（LLM 增强分析）  │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 数据导入

#### 4.2.1 TraceView 结果导入

- 支持导入 `semantic_result.json` + `popo_result.json` 文件对
- 自动解析实体流、名词信号、文档结构树
- 数据校验与完整性检查
- 支持批量导入多份文档

#### 4.2.2 原始文档导入

- 支持格式：PDF、Markdown、TXT、DOCX、EPUB
- 内置轻量解析器（基于 `pdfium` / `calamine` 等 Rust crate）
- 无 TraceView 结果时，使用内置 NLP 管线（可选调用外部 NLP 服务）

#### 4.2.3 项目管理

- 每个项目独立文件夹（自包含）
- 项目结构：
  ```
  project/
  ├── project.json          # 项目元数据
  ├── source/               # 原始文档
  ├── traceview/            # TraceView 结果
  │   ├── semantic_result.json
  │   └── popo_result.json
  ├── analysis/             # 分析结果
  │   ├── report.json
  │   └── dimensions/
  ├── exports/              # 导出文件
  └── duckdb/               # 本地数据库
      └── project.duckdb
  ```

### 4.3 分析引擎

分析引擎是 NarrativeStudio 的核心，分为五大类：

#### 4.3.1 文本诊断（Text Diagnostics）

| 功能 | 输入 | 输出 | 实现方式 |
|------|------|------|---------|
| 拼写错误查找 | 语义块文本 | 错误位置、建议修正、置信度 | 本地词典 + 编辑距离 |
| 陈词滥调查找 | 语义块文本 | 匹配短语、位置、替代表达 | 语料库短语匹配 |
| 重复短语检测 | 全文 | 重复短语列表、频率、位置 | n-gram 频率统计 |
| 副词/形容词检测 | 语义块文本 | 频率排行、重复位置 | POS 标注 + 频率统计 |

#### 4.3.2 统计分析（Statistical Analysis）

| 功能 | 输入 | 输出 | 实现方式 |
|------|------|------|---------|
| 句子统计 | 全文 | 句数、平均句长、分布 | 分句 + 统计 |
| 可读性评分 | 全文 | Flesch 评分、难度等级 | Flesch Reading Ease |
| 对话/叙事比例 | 语义块 | 对话占比、章节分布 | 对话检测（引号/格式） |

#### 4.3.3 叙事分析（Narrative Analysis）

| 功能 | 输入 | 输出 | 实现方式 |
|------|------|------|---------|
| 叙事弧线分析 | 文档结构 + 实体流 | 弧线图、情节点列表 | 结构匹配 + LLM 增强 |
| 情节分析 | 实体流 + 名词信号 | 情节线图、完整性评分 | 实体关系图 + LLM |
| 角色分析 | PERSON 实体 + 对话 | 角色档案、关系图、弧线 | 实体聚类 + 对话归属 |
| 节奏分析 | 文档结构 + 场景 | 节奏曲线、紧张度 | 场景密度 + 句长变化 |
| 冲突分析 | 全文 + 实体 | 冲突类型、强度曲线 | LLM 语义分析 |
| 主题分析 | 名词信号 + 全文 | 主题列表、强度图 | 名词信号聚类 + LLM |
| 场景/设定分析 | LOCATION 实体 + 名词信号 | 场景清单、一致性 | 实体 + 时间线 |
| 故事结构 | 文档结构树 | 结构匹配报告 | 经典模型匹配 |

#### 4.3.4 内容分析（Content Analysis）

| 功能 | 输入 | 输出 | 实现方式 |
|------|------|------|---------|
| 显式语言检测 | 全文 | 匹配词汇、位置、分级 | 敏感词库 + 上下文 |
| 内容敏感性 | 全文 | 暴力/性/仇恨检测 | LLM 分类 |

#### 4.3.5 评估建议（Assessment & Recommendations）

| 功能 | 输入 | 输出 | 实现方式 |
|------|------|------|---------|
| 总体评估 | 所有维度结果 | 综合评分、雷达图 | 加权聚合 |
| 关键建议 | 所有维度结果 | 优先级建议列表 | 规则 + LLM |
| 不一致性检测 | 实体流 + 时间线 | 问题清单、严重等级 | 规则 + LLM |
| 最终审查清单 | 所有维度结果 | 可勾选清单 | 规则引擎 |

### 4.4 报告与可视化

#### 4.4.1 报告结构

```
NarrativeStudio 诊断报告
├── 1. 总体评估 (Overall Assessment)
│   ├── 综合评分 (0-100)
│   ├── 维度雷达图 (5 维)
│   └── 总体评语
├── 2. 关注点 (Points of Concern)
│   ├── 问题清单 (按严重等级排序)
│   ├── 每条问题：位置 + 描述 + 建议
│   └── 问题类型分布图
├── 3. 叙事分析 (Narrative Analysis)
│   ├── 叙事弧线图
│   ├── 情节线图
│   ├── 角色关系图
│   ├── 节奏曲线
│   ├── 冲突强度图
│   ├── 主题强度图
│   └── 故事结构匹配
├── 4. 文本诊断 (Text Diagnostics)
│   ├── 拼写错误列表
│   ├── 陈词滥调列表
│   ├── 重复短语列表
│   └── 副词/形容词频率
├── 5. 统计分析 (Statistical Analysis)
│   ├── 句子统计
│   ├── 可读性评分
│   └── 对话/叙事比例
├── 6. 内容分析 (Content Analysis)
│   ├── 敏感内容标记
│   └── 内容分级
├── 7. 关键建议 (Key Recommendations)
│   ├── 优先级建议列表
│   └── 修改示例
└── 8. 最终审查清单 (Final Review Checklist)
    └── 可勾选检查项
```

#### 4.4.2 可视化图表

| 图表类型 | 用途 | 技术实现 |
|---------|------|---------|
| 雷达图 | 总体评估 5 维 | SVG 渲染 (Tauri WebView) |
| 折线图 | 节奏曲线、紧张度 | SVG 渲染 |
| 关系图 | 角色关系、情节线 | 自定义 SVG 环形布局（不用 D3、不用力导向：迭代布局每次收敛不同，结果不可复现也无法单测） |
| 柱状图 | 词频、实体分布 | SVG 渲染 |
| 时间线 | 时间标记位置、日期排序 | 位置条 + 表格（不用 SVG：标记是一条文档上的单向刻度，没有第二维可画） |
| 热力图 | 章节 × 维度矩阵 | SVG 渲染 |

#### 4.4.3 统计界面双视图

统计界面分为两种视图模式，用户可通过顶部切换栏自由切换：

```
┌─────────────────────────────────────────────────────────────────┐
│  [ 单项目视图 ]  [ 多项目对比视图 ]          筛选: [全部维度 ▼]  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  视图内容区（根据选中模式渲染）                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

##### 视图一：单项目视图（Single Project View）

聚焦**一个 TraceView 成果项目**的深度分析，展示该项目的完整统计画像。

**布局结构：**

```
┌─────────────────────────────────────────────────────────────────┐
│  项目：《XXX》 | 文档：3 份 | 总字数：128,450 | 章节：42         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────┐  ┌──────────────────────────────────┐ │
│  │  综合评分雷达图        │  │  维度得分条形图                    │ │
│  │  (5 维雷达)           │  │  (19 个维度横向条形，按分排序)      │ │
│  │                      │  │                                  │ │
│  │     语言质量 82       │  │  可读性        ████████████ 82   │ │
│  │    /          \      │  │  角色分析      ██████████░░ 74   │ │
│  │  结构    节奏        │  │  叙事弧线      █████████░░░ 71   │ │
│  │   74      68         │  │  主题分析      ████████░░░░ 65   │ │
│  │    \          /      │  │  节奏分析      ███████░░░░░ 58   │ │
│  │  角色    主题        │  │  ...                        ... │ │
│  │   74      65         │  │                                  │ │
│  └──────────────────────┘  └──────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  节奏曲线（章节 × 紧张度）                                    ││
│  │  ┌────────────────────────────────────────────────────────┐ ││
│  │  │  100│        *                                          ││
│  │  │   80│     *    *         *                              ││
│  │  │   60│  *       *    *      *    *                       ││
│  │  │   40│*           *         *       *                    ││
│  │  │   20│                                                   ││
│  │  │   00├──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──▶ 章节         ││
│  │  │      1  5  10 15 20 25 30 35 40 42                      ││
│  │  └────────────────────────────────────────────────────────┘ ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌──────────────────────┐  ┌──────────────────────────────────┐│
│  │  实体分布饼图          │  │  名词信号 Top-20 词云             ││
│  │  (PERSON/ORG/LOC/...)│  │                                  ││
│  └──────────────────────┘  └──────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  章节 × 维度 热力图                                           ││
│  │  ┌────┬────┬────┬────┬────┬────┬────┬────┐                ││
│  │  │    │ Ch1│ Ch2│ Ch3│ Ch4│ Ch5│ ...│ Ch42│                ││
│  │  ├────┼────┼────┼────┼────┼────┼────┼────┤                ││
│  │  │节奏│ 🟧 │ 🟨 │ 🟩 │ 🟨 │ 🟧 │ ...│ 🟩 │                ││
│  │  │角色│ 🟩 │ 🟩 │ 🟨 │ 🟩 │ 🟩 │ ...│ 🟨 │                ││
│  │  │冲突│ 🟨 │ 🟧 │ 🟩 │ 🟩 │ 🟨 │ ...│ 🟧 │                ││
│  │  │主题│ 🟩 │ 🟩 │ 🟩 │ 🟨 │ 🟩 │ ...│ 🟩 │                ││
│  │  └────┴────┴────┴────┴────┴────┴────┴────┘                ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  文本诊断统计                                                  ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      ││
│  │  │ 拼写错误  │ │ 陈词滥调  │ │ 重复短语  │ │ 副词密度  │      ││
│  │  │   23 处  │ │   15 处  │ │   8 处   │ │  3.2%   │      ││
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘      ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

**单项目视图包含的图表：**

| 区域 | 图表 | 数据源 | 说明 |
|------|------|--------|------|
| 顶部摘要栏 | 关键指标卡片 | 聚合 | 项目名、文档数、总字数、章节数 |
| 左上 | 雷达图 | 5 维聚合分 | 语言/结构/角色/节奏/主题 |
| 右上 | 横向条形图 | 19 维度得分 | 按分数降序，颜色编码严重度 |
| 中部 | 节奏曲线 | 章节 × 紧张度 | 折线图，X=章节序号，Y=紧张度 0-100 |
| 叙事分析 | 角色关系图 | 人物实体同块共现 | 环形布局 SVG，节点=提及次数（开方缩放），线=共现次数 |
| 中左 | 实体分布饼图 | NER 实体流 | 按 category 分组占比 |
| 中右 | 名词信号词云 | noun_signals | 按 score 加权字号，Top-20 |
| 中下 | 热力图 | 章节 × 维度矩阵 | 颜色=得分，绿→黄→红 |
| 底部 | 诊断统计卡片 | 文本诊断 | 4 个关键计数卡片 |

##### 视图二：多项目对比视图（Multi-Project Comparison View）

将**多个 TraceView 成果项目**并排对比，形成可比较的可视化效果。

**顶部控制栏：**

```
┌─────────────────────────────────────────────────────────────────┐
│  已选项目: [✓《项目A》] [✓《项目B》] [✓《项目C》] [+ 添加项目]    │
│  对比维度: [✓ 总体] [✓ 叙事] [✓ 文本] [✓ 统计] [全部]           │
└─────────────────────────────────────────────────────────────────┘
```

**7 个对比图表区域：**

| 序号 | 图表 | 类型 | 对比方式 | 说明 |
|------|------|------|---------|------|
| ① | 综合评分对比 | 叠加雷达图 | 多项目同图叠加 | 每个项目一条多边形，不同颜色/线型 |
| ② | 维度得分对比 | 分组柱状图 | 每维度 N 根柱子 | N=项目数，颜色区分项目 |
| ③ | 节奏曲线对比 | 多线折线图 | X 轴归一化 0-100% | 解决不同项目章节数不同的问题 |
| ④ | 实体分布对比 | 堆叠条形图 | 每项目一行，按 category 堆叠 | 直观比较实体类型占比差异 |
| ⑤ | 文本诊断对比 | 数值表格 + 柱状图 | 标准化指标（每万字） | 消除篇幅差异，支持排序 |
| ⑥ | 热力图对比 | 并排热力图 | 每项目独立热力图并排 | 快速识别各项目章节质量分布差异 |
| ⑦ | 名词信号对比 | 并排词云 + 交集 | 并排词云 + 共同术语高亮 | 发现跨项目共同关注的领域术语 |

**① 叠加雷达图（综合评分对比）：**

```
┌─────────────────────────────────────────────────────────┐
│                  语言质量                                 │
│                   100                                    │
│                 ╱  A   ╲                                 │
│               ╱  ───    ╲                                │
│             ╱  ╱  B  ╲   ╲                               │
│           ╱  ╱  ─────  ╲  ╲                              │
│         ╱  ╱    C      ╲   ╲                             │
│       结构╱  ───────     ╲  节奏                          │
│         ╲                ╱                                │
│           ╲    B       ╱                                  │
│             ╲  ───   ╱                                    │
│               ╲     ╱                                     │
│                 角色   主题                                │
│  图例: ── A(《项目A》)  ── B(《项目B》)  ── C(《项目C》)   │
└─────────────────────────────────────────────────────────┘
```

**② 分组柱状图（维度得分对比）：**

```
  90│  ██
  80│  ██  ██
  70│  ██  ██  ██         ██  ██  ██
  60│  ██  ██  ██  ██  ██  ██  ██  ██  ██  ██
  50│  ██  ██  ██  ██  ██  ██  ██  ██  ██  ██
  40│  ██  ██  ██  ██  ██  ██  ██  ██  ██  ██
    ├────┬────┬────┬────┬────┬────┬────┬────┬────┬────
    可读 角色 弧线 主题 节奏 冲突 场景 对话 陈词 拼写
    每维度 3 根柱子: A(蓝) B(橙) C(绿)
```

**③ 多线折线图（节奏曲线对比）：**

```
  100│      A: *──*──*
   80│   B: *──*──*──*──*
   60│      C: *──*──*──*
   40│   B: *──*──*──*
   20│      C: *──*──*
   00├──┬──┬──┬──┬──┬──┬──┬──┬──┬──▶ 章节(归一化 0-100%)
      0  10 20 30 40 50 60 70 80 90 100
  注: X 轴归一化为百分比，解决不同项目章节数不同的问题
```

**④ 堆叠条形图（实体分布对比）：**

```
  项目A │████████████████████████████████│██│█│
        │ PERSON  ORG  LOC  DATE  OTHER
  项目B │██████████████████│████████│██│████│
  项目C │████████████│██████████████│██│██████│
```

**⑤ 数值表格（文本诊断对比）：**

| 指标 | 项目A | 项目B | 项目C | 均值 |
|------|-------|-------|-------|------|
| 拼写错误/万字 | 23 | 45 | 12 | 26.7 |
| 陈词滥调/万字 | 15 | 8 | 22 | 15.0 |
| 重复短语/万字 | 8 | 12 | 5 | 8.3 |
| 副词密度% | 3.2 | 4.8 | 2.1 | 3.4 |
| 平均句长 | 18.2 | 22.5 | 15.3 | 18.7 |
| 可读性评分 | 72 | 58 | 81 | 70.3 |
| 对话占比% | 35 | 28 | 42 | 35.0 |

**⑥ 并排热力图（章节 × 维度对比）：**

```
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   项目 A     │ │   项目 B     │ │   项目 C     │
│  ┌──────────┐│ │  ┌──────────┐│ │  ┌──────────┐│
│  │Ch1 Ch2..││ │  │Ch1 Ch2..││ │  │Ch1 Ch2..││
│  │🟧 🟨 🟩 ││ │  │🟨 🟧 🟨 ││ │  │🟩 🟩 🟨 ││
│  │🟩 🟩 🟨 ││ │  │🟩 🟨 🟧 ││ │  │🟩 🟩 🟩 ││
│  │🟨 🟧 🟩 ││ │  │🟨 🟩 🟨 ││ │  │🟩 🟨 🟩 ││
│  └──────────┘│ │  └──────────┘│ │  └──────────┘│
└──────────────┘ └──────────────┘ └──────────────┘
```

**⑦ 并排词云 + 共同术语（名词信号对比）：**

```
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   项目 A     │ │   项目 B     │ │   项目 C     │
│  绝缘电阻    │ │  切削速度    │ │  热处理      │
│  绝缘电阻    │ │  刀具        │ │  金相        │
│   测量       │ │  切削速度    │ │  热处理      │
│  测量  绝缘  │ │   刀具       │ │   金相       │
└──────────────┘ └──────────────┘ └──────────────┘
共同术语: [绝缘电阻] [测量] → 高亮标记
```

**多项目对比的关键设计原则：**

| 原则 | 说明 |
|------|------|
| **归一化** | 不同项目篇幅不同，所有指标需标准化（每万字、百分比、0-100 分） |
| **颜色一致性** | 同一项目在所有图表中使用相同颜色（项目 A=蓝、B=橙、C=绿...） |
| **X 轴对齐** | 节奏曲线等按章节百分比归一化，而非绝对章节号 |
| **可筛选** | 用户可选择要对比的维度子集，避免信息过载 |
| **可排序** | 表格视图支持按任意列排序，快速定位最优/最差项目 |
| **差异高亮** | 自动标记项目间差异最大的维度（如某项目节奏分显著低于其他） |
| **上限** | 建议同时对比 ≤ 5 个项目，超过时提示用户筛选 |

### 4.5 Knowsphere 导出契约（NarratSphere 数据接口）

NarrativeStudio 是**数据构建层**，NarratSphere 是**聚合与呈现层**。两者的唯一接口是
机器可读的 `.knowsphere.json` 文件（schema v1.0，由 NarratSphere 定义并负责解析、迁移、
归一化与校验）。本节记录 NarrativeStudio 侧的导出契约。

#### 4.5.1 边界规则

| 规则 | 说明 |
|------|------|
| 单向文件契约 | Studio 只写文件；不共享代码、不共享存储、不回调 |
| 不导出布局 | 节点**从不**带 `position`——球壳布局是 NarratSphere 的职责；只有叙事元素按 schema 要求携带大气层内坐标（由类型高度 + 黄金角确定性推得） |
| 不导出 layers / view | 省略 `layers` 即选用 NarratSphere 内置六层，层级身份（名称、半径、颜色）单一事实源在消费方；`view` 是会话数据不是内容 |
| 先自检后落盘 | 导出前在 Rust 侧按同一 schema 校验（`KnowsphereDoc::validate`），不通过则拒绝写文件；NarratSphere 永远收不到本侧已知损坏的文件 |
| 确定性输出 | 同一分析 + 同一时间戳 → 字节一致的文件（serde 字段顺序固定、2 空格缩进、结尾换行） |
| 未知字段兼容 | Studio 不写入 schema 之外的顶层字段；来源信息放在约定位置 `metadata.custom.studio` |

#### 4.5.2 语义映射表

| Studio 数据 | Knowsphere 落点 |
|-------------|-----------------|
| 实体身份（全部抽取实体，口径与分析层一致；PERSON/ORG/LOCATION/FACILITY/PRODUCT） | L2 `entity` 节点，`weight` ← 提及频次归一 |
| 实体共现（T1 `A ↔ B`） | `association` 边，`strength` ← 共现次数归一（端点必须已存在） |
| 主题（theme.themes） | L3 `concept` 节点 + 天空 `cloud`（中弱关联）系泊于该节点 |
| 地点（setting.places） | L2 `entity` 节点；与同名实体**合并为同一节点** |
| 时间线跨度 | 一个 L2 `fact` 节点（`时间跨度 first → last`） |
| 评估建议 | L2 `observation` 节点，tag 携带 severity |
| 主线·sustained 情节线 | 天空 `rain_band`（持续反馈） |
| 主线·其他形态 | 天空 `tethered_balloon`（强关联） |
| 支线（intermittent/burst/late_arrival） | 天空 `satellite`（远端宏观） |
| setup_only 线程 | 天空 `free_balloon`（弱关联） |
| 转折点 | 天空 `tornado`（动态强关联），`stability = 1 - change`，tether 到登场线程的节点 |
| DATE/NUMBER/UNKNOWN 实体、minor 线程 | 不导出（噪声） |

#### 4.5.3 稳定实体身份

导出文件中的实体节点 id 是**内容派生的稳定 id**（`ent:<16 hex>`，
由 `studio-storage::entity_registry` 按 (归一化文本, 类别) 签发，v13 迁移落库）。
同一人物在重复导入、不同项目中得到同一 id，因此 NarratSphere 把新导出**增量合并**进
用户手工编辑过的知识球时，可以按 id 对齐节点、边与系泊，而不是整文件覆盖。
主题/观察/边/叙事元素的 id 用同一思路的内容哈希派生，保证跨导出稳定。

#### 4.5.4 版本与实现位置

- 契约 schema 版本：`1.0`（`studio_core::knowsphere::KNOWSPHERE_VERSION`）。
- 契约镜像与自检：`crates/studio-core/src/knowsphere.rs`。
- 映射渲染器：`crates/studio-report/src/knowsphere.rs`（纯函数，与 Markdown 渲染器同级）。
- 导出命令：`export_knowsphere`（`commands/report.rs`）+ 分析页「导出知识球」按钮。
- 来源元数据：`metadata.custom.studio = { projectId, studioVersion, analysisVersion, sourceHash }`，
  其中 `sourceHash` 与分析缓存使用同一个内容哈希，可追溯导出对应的确切文本。

契约变更流程：NarratSphere 侧 schema 升版 → Studio 同步更新镜像与 `KNOWSPHERE_VERSION`
→ 映射变化写入本节映射表 → 若语义变化（某维度换落点），同一 PR 更新本节与渲染器测试。

---


---

## 5. 分析维度矩阵

### 5.1 维度 × 数据源映射

| 分析维度 | TraceView 数据源 | 内置 NLP | LLM 增强 | 优先级 |
|---------|-----------------|---------|---------|--------|
| 总体评估 | 聚合 | 聚合 | 综合评语 | P0 |
| 关注点 | 实体流 + 名词信号 | 规则检测 | 问题描述生成 | P0 |
| 叙事弧线 | 文档结构树 | 结构匹配 | 弧线识别 | P0 |
| 情节分析 | 实体流 + 名词信号 | 实体关系 | 情节线提取 | P0 |
| 角色分析 | PERSON 实体 | 对话归属 | 角色弧线 | P0 |
| 节奏分析 | 文档结构 | 句长统计 | 紧张度评估 | P1 |
| 冲突分析 | 全文 | 关键词 | 冲突识别 | P1 |
| 主题分析 | 名词信号 | 词频统计 | 主题提取 | P1 |
| 场景/设定 | LOCATION 实体 | 时间线 | 一致性检查 | P1 |
| 作者声音 | 全文 | 风格统计 | 文风分析 | P2 |
| 对话/叙事 | 语义块 | 对话检测 | 比例分析 | P1 |
| 可读性 | 全文 | Flesch 算法 | - | P0 |
| 陈词滥调 | 语义块 | 短语匹配 | 替换建议 | P1 |
| 显式语言 | 全文 | 敏感词库 | 分级评估 | P1 |
| 重复短语 | 全文 | n-gram | - | P0 |
| 副词/形容词 | 语义块 | POS 统计 | - | P0 |
| 拼写错误 | 语义块 | 词典 + 编辑距离 | - | P0 |
| 关键建议 | 聚合 | 规则引擎 | 建议生成 | P0 |
| 故事结构 | 文档结构树 | 模型匹配 | 结构建议 | P1 |

### 5.2 优先级定义

- **P0（MVP）**：核心功能，首个版本必须实现
- **P1（V1.1）**：重要功能，第二个版本实现
- **P2（V1.2+）**：增强功能，后续版本迭代

---

## 6. 技术架构

### 6.1 技术栈

| 层级 | 技术选型 | 说明 |
|------|---------|------|
| 桌面框架 | **Tauri 2.x** | Rust 后端 + WebView 前端 |
| 后端语言 | **Rust** | 核心分析引擎、数据处理 |
| 前端框架 | **React 18 + TypeScript** | UI 渲染、图表交互 |
| 本地数据库 | **DuckDB** | 规范化存储（遵循 NarrativeOS 规则 #3） |
| 图表渲染 | **SVG + D3.js** | 轻量级可视化 |
| 状态管理 | **Zustand** | 前端状态 |
| 样式 | **Tailwind CSS** | 原子化 CSS |
| 包管理 | **pnpm** (前端) + **Cargo** (Rust) | 前后端独立管理 |

### 6.2 架构分层

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (WebView)                                         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  React 18 + TypeScript + Tailwind CSS                   ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  ││
│  │  │ 报告视图  │ │ 图表组件  │ │ 项目面板  │ │ 设置面板  │  ││
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  ││
│  │  ┌──────────────────────────────────────────────────┐  ││
│  │  │  Zustand Store (项目状态、分析状态、UI 状态)       │  ││
│  │  └──────────────────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  Tauri IPC Bridge                                           │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  Commands (Rust → Frontend)                             ││
│  │  • project_* (项目 CRUD)                                ││
│  │  • import_* (数据导入)                                   ││
│  │  • analyze_* (分析执行)                                  ││
│  │  • report_* (报告生成/导出)                              ││
│  │  • settings_* (设置管理)                                 ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐   ││
│  │  │ 数据导入层   │ │ 分析引擎层   │ │ 报告生成层       │   ││
│  │  │             │ │             │ │                 │   ││
│  │  │ • TraceView │ │ • 文本诊断   │ │ • 报告聚合      │   ││
│  │  │   解析器    │ │ • 统计分析   │ │ • 可视化数据    │   ││
│  │  │ • 文档解析器│ │ • 叙事分析   │ │ • 导出 (MD/PDF)│   ││
│  │  │ • 项目存储  │ │ • 内容分析   │ │                 │   ││
│  │  └─────────────┘ │ • 评估建议   │ └─────────────────┘   ││
│  │                   └─────────────┘                        ││
│  │  ┌─────────────────────────────────────────────────┐    ││
│  │  │  数据底座：DuckDB                                │    ││
│  │  │  • 项目元数据                                    │    ││
│  │  │  • 实体表 / 名词信号表 / 语义块表                 │    ││
│  │  │  • 分析结果表 / 报告表                           │    ││
│  │  └─────────────────────────────────────────────────┘    ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  可选云层 (Cloud Optional)                                   │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  • LLM API 调用（叙事分析增强）                          ││
│  │  • 不破坏本地可用性（规则 #6）                           ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### 6.3 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 运行时隔离 | Rust 后端 + WebView 前端 | 遵循 NarrativeOS 规则 #1 |
| 存储 | DuckDB | 遵循 NarrativeOS 规则 #3，列式分析性能优 |
| IPC | Tauri Commands | 遵循 NarrativeOS 规则 #4，不走共享依赖 |
| 插件 | API 合约边界 | 遵循 NarrativeOS 规则 #5 |
| 云能力 | 可选层 | 遵循 NarrativeOS 规则 #6，本地优先 |
| 代码组织 | Rust workspace crates | 遵循 NarrativeOS 规则 #7 |
| 接口 | 类型化接口 + 显式契约 | 遵循 NarrativeOS 规则 #8 |

### 6.4 数据流

```
用户操作 (前端)
  │
  ▼
Tauri Command (IPC)
  │
  ▼
Rust 后端处理
  ├── 1. 数据导入 → DuckDB 写入
  ├── 2. 分析引擎 → 多维度分析
  │     ├── 本地规则引擎（确定性）
  │     ├── 统计计算（确定性）
  │     └── LLM 调用（可选，增强）
  ├── 3. 结果写入 DuckDB
  └── 4. 报告聚合 → 返回前端
  │
  ▼
前端渲染 (React + SVG 图表)
```

### 6.5 分析管线与后台任务架构

#### 6.5.1 真实数据画像（基于 Hardwired 项目实测）

| 数据文件 | 大小 | 规模 | 关键指标 |
|---------|------|------|---------|
| `semantic_result.json` | 6.5 MB | 326,465 行 | 300 blocks, 12,804 tokens, 54,570 chars |
| `popo_result.json` | 212 KB | 4,726 行 | 52 tree nodes, 28 顶层章节 |
| `enriched_result.json` | 224 KB | - | 616 entities, 2 relations |

**实体分布（enriched）：**

| 类别 | 数量 | 占比 |
|------|------|------|
| UNKNOWN | 378 | 61.4% |
| PERSON | 205 | 33.3% |
| DATE | 19 | 3.1% |
| ORGANIZATION | 5 | 0.8% |
| LOCATION | 5 | 0.8% |
| FACILITY | 3 | 0.5% |
| PRODUCT | 1 | 0.2% |

**名词信号：** 552 个唯一词，2,609 次总出现，Top-5: analysis(438), story(176), conflict(77), character(72), Plot(70)

**块分布：** 242 text + 56 title + 2 empty；279/300 含实体，298/300 含名词信号；51 个唯一 section_path

**Popo 结构树（28 个顶层章节）：**
Premise → Genre → Overall assessment → Story strengths → Chief opportunities → Word count → Narrative arc (6 节) → Plot (7 节) → Story elements → Character analysis (4 子节) → Character dynamics → POV → Pacing → Conflict (5 子节) → Theme → Author voice → Other

#### 6.5.2 分析任务分类

所有分析任务按**计算特征**分为三类，决定执行策略：

| 类别 | 特征 | 延迟 | 示例 |
|------|------|------|------|
| **T0: 纯统计** | 确定性、无 I/O、CPU 密集 | < 100ms | 词频、句长、POS 分布、实体计数、n-gram |
| **T1: 结构分析** | 确定性、树遍历、规则匹配 | 100ms ~ 2s | 章节结构提取、对话检测、可读性评分、时间线 |
| **T2: LLM 增强** | 非确定性、网络 I/O、token 消耗 | 5s ~ 60s | 叙事弧线、角色弧线、主题提取、建议生成 |

**T0 任务清单（全部本地，无需 LLM）：**

| 任务 | 输入 | 输出 | 算法 |
|------|------|------|------|
| 词频统计 | tokens[] | word → count | HashMap 计数 |
| 句长分布 | blocks[].content | 句长直方图 | 分句 + 计数 |
| POS 分布 | tokens[].pos | pos → count | HashMap 计数 |
| 副词/形容词密度 | tokens[].pos | 占比 | 过滤 RB/JJ 计数 |
| 实体聚合 | entities[] | category → count, top-N | 分组 + 排序 |
| 名词信号 Top-N | noun_signals[] | top-N by score | 排序 |
| 重复短语 | blocks[].content | n-gram 重复列表 | 3-gram 窗口 + 计数 |
| 拼写检查 | tokens[] | 错误列表 | 词典 + 编辑距离 ≤ 2 |
| 对话检测 | blocks[].content | 对话占比 | 引号 + 对话标记正则 |
| 可读性评分 | tokens[], blocks[] | Flesch 分数 | 公式计算 |

**T1 任务清单（本地规则引擎）：**

| 任务 | 输入 | 输出 | 算法 |
|------|------|------|------|
| 章节结构提取 | popo tree | 章节层级 + 字数 | 树遍历 |
| 节奏曲线 | blocks[] × section | 章节 → 紧张度 | 句长变异 + 感叹号密度 + 短句比 |
| 陈词滥调检测 | blocks[].content | 匹配列表 | 短语库 + 模糊匹配 |
| 显式语言检测 | blocks[].content | 标记列表 | 敏感词库 + 正则 |
| 时间线一致性 | blocks[].content | 标记清单 + 倒回/跳跃 | 时间表达式扫描 + 日历排序 |
| 故事结构匹配 | popo tree | 结构类型 | 模式匹配（三幕/英雄之旅等） |

**T2 任务清单（LLM 增强，可选）：**

| 任务 | Prompt 规模 | 输入上下文 | 输出 |
|------|-----------|-----------|------|
| 叙事弧线识别 | ~2K tokens | 章节摘要 + 结构树 | 弧线类型 + 转折点 |
| 角色弧线分析 | ~3K tokens | PERSON 实体 + 相关段落 | 角色发展轨迹 |
| 主题提取 | ~2K tokens | Top-50 名词信号 + 章节摘要 | 主题列表 + 证据 |
| 冲突分析 | ~3K tokens | 冲突相关段落 | 冲突类型 + 强度 |
| 情节线提取 | ~4K tokens | 实体关系 + 章节内容 | 情节线 + 子情节 |
| 作者声音 | ~2K tokens | 风格统计 + 样本段落 | 文风描述 |
| 关键建议 | ~3K tokens | 所有 T0/T1 结果摘要 | 优先级排序建议 |
| 关注点生成 | ~3K tokens | 异常检测结果 | 问题描述 + 位置 |

#### 6.5.3 后台任务管线架构

**核心原则：所有计算任务在后台 tokio 任务中执行，前端永不阻塞。**

```
┌─────────────────────────────────────────────────────────────────────┐
│  Frontend (WebView)                                                 │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  UI 状态: idle → loading → partial → complete                  │ │
│  │  进度条: [████████░░░░░░░░░░] 5/12 tasks (42%)                 │ │
│  │  结果流: 每完成一个任务立即渲染对应图表区域                       │ │
│  └────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│  Tauri IPC (事件通道)                                               │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  请求: analyze_project(project_id, options) → task_id          │ │
│  │  事件: task_progress { task_id, current, total, label }        │ │
│  │  事件: task_result   { task_id, dimension, data }              │ │
│  │  事件: task_complete { task_id, summary }                      │ │
│  │  取消: cancel_analysis(task_id)                                │ │
│  └────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│  Backend (Rust) — tokio 异步运行时                                   │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  AnalysisOrchestrator                                          │ │
│  │  ┌──────────────────────────────────────────────────────────┐  │ │
│  │  │  1. 数据加载 (spawn)                                      │  │ │
│  │  │     semantic_result.json → 内存 (serde_json, ~50ms)      │  │ │
│  │  │     popo_result.json → 内存 (~5ms)                       │  │ │
│  │  │     enriched_result.json → 内存 (~5ms)                   │  │ │
│  │  │     → 写入 DuckDB (首次导入时)                            │  │ │
│  │  ├──────────────────────────────────────────────────────────┤  │ │
│  │  │  2. T0 统计任务 (并行, join_all)                          │  │ │
│  │  │     ┌─────────┬─────────┬─────────┬─────────┐           │  │ │
│  │  │     │ 词频    │ 句长    │ POS     │ 实体    │  ...      │  │ │
│  │  │     │ <10ms   │ <10ms   │ <5ms    │ <10ms   │           │  │ │
│  │  │     └─────────┴─────────┴─────────┴─────────┘           │  │ │
│  │  │     全部完成 → 发射 task_result 事件                      │  │ │
│  │  ├──────────────────────────────────────────────────────────┤  │ │
│  │  │  3. T1 结构任务 (并行, join_all)                          │  │ │
│  │  │     ┌──────────────┬──────────────┬──────────────┐       │  │ │
│  │  │     │ 章节结构     │ 节奏曲线     │ 对话检测     │ ...   │  │ │
│  │  │     │ ~50ms       │ ~100ms       │ ~50ms        │       │  │ │
│  │  │     └──────────────┴──────────────┴──────────────┘       │  │ │
│  │  │     全部完成 → 发射 task_result 事件                      │  │ │
│  │  ├──────────────────────────────────────────────────────────┤  │ │
│  │  │  4. T2 LLM 任务 (串行/限流, 可选)                         │  │ │
│  │  │     ┌──────────────────────────────────────────────┐     │  │ │
│  │  │     │ 叙事弧线 → 角色弧线 → 主题 → 冲突 → 建议     │     │  │ │
│  │  │     │  (5s)       (8s)       (5s)    (8s)   (10s)  │     │  │ │
│  │  │     └──────────────────────────────────────────────┘     │  │ │
│  │  │     每个完成 → 立即发射 task_result 事件                  │  │ │
│  │  │     用户未配置 LLM → 跳过此阶段                           │  │ │
│  │  └──────────────────────────────────────────────────────────┘  │ │
│  │                                                                │ │
│  │  取消: CancellationToken 贯穿所有阶段                           │ │
│  │  缓存: 结果写入 DuckDB，相同输入不重复计算                       │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

#### 6.5.4 LLM 使用分级

| 级别 | 名称 | 触发条件 | 任务数 | 预估耗时 | Token 消耗 |
|------|------|---------|--------|---------|-----------|
| **L0** | 纯本地 | 默认 | 10 T0 + 6 T1 | < 2s | 0 |
| **L1** | 轻量增强 | 用户开启"快速 AI" | L0 + 3 T2 | ~20s | ~15K |
| **L2** | 完整增强 | 用户开启"深度 AI" | L0 + 6 T1 + 8 T2 | ~60s | ~50K |

**LLM 调用策略：**

```rust
// LLM 调用器（studio-analysis crate）
pub struct LlmClient {
    provider: LlmProvider,       // OpenAI / Anthropic / Ollama(本地)
    model: String,
    max_concurrent: usize,       // 默认 1（串行），避免 rate limit
    timeout: Duration,           // 单任务 60s
    retry: u32,                  // 最多重试 2 次
}

// 任务调度
pub struct AnalysisTask {
    id: TaskId,
    dimension: Dimension,
    tier: TaskTier,              // T0 / T1 / T2
    input: AnalysisInput,
    cancel: CancellationToken,
}
```

**关键约束：**
- LLM 调用**串行执行**（max_concurrent=1），避免 API rate limit
- 每个 LLM 任务独立超时（60s），超时后标记为 `degraded`，不阻塞后续任务
- 用户可随时取消，已完成的 T0/T1 结果保留
- 本地 Ollama 支持（`ollama://localhost:11434`），完全离线可用

#### 6.5.5 进度报告与前端渲染策略

**前端渲染原则：渐进式加载，每完成一个维度立即渲染。**

```typescript
// 前端 Zustand Store
interface AnalysisState {
  status: 'idle' | 'loading' | 'partial' | 'complete' | 'error';
  progress: { current: number; total: number; label: string };
  results: Record<string, DimensionResult>;  // 按维度 key 存储
  errors: Record<string, string>;            // 失败的维度
}

// 事件监听
listen('task_progress', (e) => {
  store.setState({ progress: e.payload });
});
listen('task_result', (e) => {
  const { dimension, data } = e.payload;
  store.setState((s) => ({
    results: { ...s.results, [dimension]: data },
    status: Object.keys(s.results).length === s.progress.total
      ? 'complete' : 'partial'
  }));
});
```

**渲染优先级（用户感知顺序）：**

| 顺序 | 维度 | 来源 | 用户感知 |
|------|------|------|---------|
| 1 | 项目摘要（字数、章节数） | T0 | 立即显示 |
| 2 | 实体分布饼图 | T0 | 100ms 内 |
| 3 | 名词信号词云 | T0 | 100ms 内 |
| 4 | 可读性 + 诊断卡片 | T0 | 100ms 内 |
| 5 | 维度得分条形图 | T0+T1 | 1s 内 |
| 6 | 节奏曲线 | T1 | 2s 内 |
| 7 | 章节热力图 | T1 | 2s 内 |
| 8 | 叙事弧线（LLM） | T2 | 10-30s |
| 9 | 角色分析（LLM） | T2 | 20-40s |
| 10 | 关键建议（LLM） | T2 | 最后完成 |

#### 6.5.6 缓存与增量分析

| 策略 | 说明 |
|------|------|
| **输入哈希** | 对 semantic_result.json + popo_result.json 计算 SHA-256，存入 DuckDB |
| **结果缓存** | 每个维度的分析结果 + 输入哈希存入 `analysis_cache` 表 |
| **增量判断** | 导入时比较哈希，未变化则直接读缓存，跳过计算 |
| **T2 单独缓存** | LLM 结果单独缓存（成本高），T0/T1 结果轻量可重算 |
| **手动刷新** | 用户可强制重新分析（清除缓存） |

```sql
CREATE TABLE analysis_cache (
    cache_key     VARCHAR PRIMARY KEY,  -- SHA256(input) + dimension
    project_id    UUID REFERENCES projects(project_id),
    dimension     VARCHAR NOT NULL,
    tier          VARCHAR NOT NULL,     -- T0 / T1 / T2
    result_json   TEXT NOT NULL,
    computed_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    input_hash    VARCHAR NOT NULL
);
```

#### 6.5.7 对 Crate 架构的影响

基于管线设计，`studio-analysis` crate 内部结构细化为：

```
studio-analysis/
├── src/
│   ├── lib.rs
│   ├── orchestrator.rs      # AnalysisOrchestrator: 任务调度、进度报告
│   ├── tasks/
│   │   ├── mod.rs
│   │   ├── t0_stats.rs      # 纯统计任务（词频、句长、POS、实体聚合...）
│   │   ├── t1_structural.rs # 结构分析任务（章节、节奏、对话、可读性...）
│   │   └── t2_llm.rs        # LLM 增强任务（弧线、角色、主题、建议...）
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── client.rs        # LLM 客户端（OpenAI/Anthropic/Ollama）
│   │   ├── prompts.rs       # Prompt 模板
│   │   └── parser.rs        # LLM 输出解析
│   ├── cache.rs             # 缓存逻辑（哈希比较、DuckDB 读写）
│   └── types.rs             # 任务类型、进度事件、结果类型
```

**新增依赖：**
- `tokio` (rt-multi-thread, macros, sync, time) — 异步运行时
- `tokio-util` — CancellationToken
- `reqwest` (json, stream) — LLM HTTP 调用
- `sha2` — 输入哈希
- `futures` — 并行任务 join_all

---

## 7. 数据模型

### 7.1 DuckDB 表结构

```sql
-- 项目元数据
CREATE TABLE projects (
    project_id    UUID PRIMARY KEY,
    name          VARCHAR NOT NULL,
    description   VARCHAR,
    genre         VARCHAR,
    language      VARCHAR DEFAULT 'zh-CN',
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 文档
CREATE TABLE documents (
    doc_id        UUID PRIMARY KEY,
    project_id    UUID REFERENCES projects(project_id),
    title         VARCHAR NOT NULL,
    source_type   VARCHAR,
    file_path     VARCHAR,
    word_count    INTEGER,
    chapter_count INTEGER,
    imported_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 语义块（来自 TraceView）
CREATE TABLE semantic_blocks (
    block_id      VARCHAR PRIMARY KEY,
    doc_id        UUID REFERENCES documents(doc_id),
    text          TEXT,
    section_path  VARCHAR,
    unit_type     VARCHAR,
    node_id       VARCHAR,
    segment_range VARCHAR,
    prefix_len    INTEGER DEFAULT 0
);

-- 实体（来自 TraceView NER）
CREATE TABLE entities (
    entity_id     UUID PRIMARY KEY,
    block_id      VARCHAR REFERENCES semantic_blocks(block_id),
    doc_id        UUID REFERENCES documents(doc_id),
    text          VARCHAR NOT NULL,
    category      VARCHAR NOT NULL,
    confidence    REAL,
    source        VARCHAR,
    keep          BOOLEAN DEFAULT TRUE,
    filter        VARCHAR,
    filter_reason VARCHAR,
    span_start    INTEGER,
    span_end      INTEGER
);

-- 名词信号（来自 TraceView）
CREATE TABLE noun_signals (
    signal_id     UUID PRIMARY KEY,
    block_id      VARCHAR REFERENCES semantic_blocks(block_id),
    doc_id        UUID REFERENCES documents(doc_id),
    text          VARCHAR NOT NULL,
    pos           VARCHAR,
    syntactic_role VARCHAR,
    score         REAL,
    span_start    INTEGER,
    span_end      INTEGER,
    evidence      JSON
);

-- 分析结果
CREATE TABLE analysis_results (
    result_id     UUID PRIMARY KEY,
    project_id    UUID REFERENCES projects(project_id),
    doc_id        UUID REFERENCES documents(doc_id),
    dimension     VARCHAR NOT NULL,
    result_data   JSON NOT NULL,
    score         REAL,
    computed_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    engine_version VARCHAR
);

-- 问题/关注点
CREATE TABLE concerns (
    concern_id    UUID PRIMARY KEY,
    project_id    UUID REFERENCES projects(project_id),
    doc_id        UUID REFERENCES documents(doc_id),
    dimension     VARCHAR,
    severity      VARCHAR,
    title         VARCHAR NOT NULL,
    description   TEXT,
    location      JSON,
    suggestion    TEXT,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 报告
CREATE TABLE reports (
    report_id     UUID PRIMARY KEY,
    project_id    UUID REFERENCES projects(project_id),
    title         VARCHAR,
    content       JSON NOT NULL,
    exported_path VARCHAR,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 7.2 Rust 核心类型

```rust
// 分析维度枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDimension {
    OverallAssessment,
    PointsOfConcern,
    NarrativeArc,
    PlotAnalysis,
    StoryElements,
    CharacterAnalysis,
    PacingAnalysis,
    ConflictAnalysis,
    ThemeAnalysis,
    SettingAnalysis,
    AuthorVoice,
    DialogueNarrative,
    Readability,
    ClichesFinder,
    ExplicitLanguage,
    RepetitivePhrases,
    AdverbsAdjectives,
    MisspellingsFinder,
    KeyRecommendations,
    StoryStructure,
}

// 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub text: String,
    pub category: EntityCategory,
    pub confidence: f32,
    pub source: String,
    pub keep: bool,
    pub filter: Option<String>,
    pub filter_reason: Option<String>,
    pub span: (usize, usize),
}

// 名词信号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NounSignal {
    pub text: String,
    pub pos: String,
    pub syntactic_role: Option<String>,
    pub score: f32,
    pub span: (usize, usize),
    pub evidence: Option<NounSignalEvidence>,
}

// 关注点/问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concern {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub location: ConcernLocation,
    pub suggestion: Option<String>,
}

// 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub dimension: AnalysisDimension,
    pub score: Option<f32>,
    pub data: serde_json::Value,
    pub concerns: Vec<Concern>,
    pub computed_at: DateTime<Utc>,
}
```

---

## 8. UI/UX 设计

### 8.1 整体布局

```
┌─────────────────────────────────────────────────────────────────────┐
│  NarrativeStudio                                    [─] [□] [✕]     │
├────────────┬────────────────────────────────────────────────────────┤
│            │  ┌──────────────────────────────────────────────────┐  │
│  项目列表   │  │  面包屑：项目名 > 文档名 > 分析维度              │  │
│            │  ├──────────────────────────────────────────────────┤  │
│  ┌──────┐  │  │                                                  │  │
│  │ 项目1│  │  │                                                  │  │
│  │ 项目2│  │  │              主内容区                             │  │
│  │ 项目3│  │  │         (根据选中维度渲染)                         │  │
│  └──────┘  │  │                                                  │  │
│            │  │                                                  │  │
│  ────────  │  │                                                  │  │
│            │  │                                                  │  │
│  分析维度   │  │                                                  │  │
│            │  │                                                  │  │
│  📊 总体评估│  │                                                  │  │
│  ⚠️ 关注点 │  │                                                  │  │
│  📈 叙事弧线│  │                                                  │  │
│  📖 情节   │  │                                                  │  │
│  👤 角色   │  │                                                  │  │
│  🎵 节奏   │  │                                                  │  │
│  ⚔️ 冲突   │  │                                                  │  │
│  🎭 主题   │  │                                                  │  │
│  📍 场景   │  │                                                  │  │
│  📝 文本诊断│  │                                                  │  │
│  📊 统计   │  │                                                  │  │
│  🔍 内容   │  │                                                  │  │
│  💡 建议   │  │                                                  │  │
│  ✅ 审查   │  │                                                  │  │
│            │  │                                                  │  │
├────────────┴────────────────────────────────────────────────────────┤
│  状态栏：项目 | 文档 | 分析状态 | DuckDB 连接                        │
└─────────────────────────────────────────────────────────────────────┘
```

### 8.2 交互设计

| 交互 | 说明 |
|------|------|
| 点击问题 | 跳转到原文位置（高亮） |
| 悬停图表 | 显示详细数据 tooltip |
| 拖拽导入 | 支持拖拽 TraceView JSON 文件到窗口 |
| 快捷键 | Ctrl+O 打开项目, Ctrl+R 重新分析, Ctrl+E 导出 |
| 暗色模式 | 跟随系统 / 手动切换 |

---

## 9. 模块划分与 Crate 结构

### 9.1 Rust Workspace 结构

```
NarrativeStudio/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── studio-core/              # 核心类型与接口
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── project.rs        # 项目模型
│   │       ├── document.rs       # 文档模型
│   │       ├── entity.rs         # 实体模型
│   │       ├── noun_signal.rs    # 名词信号模型
│   │       ├── analysis.rs       # 分析维度与结果
│   │       ├── concern.rs        # 关注点/问题
│   │       └── report.rs         # 报告模型
│   │
│   ├── studio-import/            # 数据导入层
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traceview.rs      # TraceView 结果解析
│   │       ├── pdf.rs            # PDF 解析
│   │       ├── markdown.rs       # Markdown 解析
│   │       └── project_store.rs  # 项目文件管理
│   │
│   ├── studio-analysis/          # 分析引擎
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs         # 分析引擎调度
│   │       ├── text_diagnostics/ # 文本诊断
│   │       ├── statistics/       # 统计分析
│   │       ├── narrative/        # 叙事分析
│   │       ├── content/          # 内容分析
│   │       └── assessment/       # 评估建议
│   │
│   ├── studio-storage/           # 数据持久化
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── duckdb.rs         # DuckDB 连接管理
│   │       ├── migrations.rs     # 表结构迁移
│   │       └── repositories/     # 数据访问层
│   │
│   ├── studio-report/            # 报告生成
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── aggregator.rs     # 报告聚合
│   │       ├── markdown.rs       # Markdown 导出
│   │       └── json.rs           # JSON 导出
│   │
│   └── studio-app/               # Tauri 应用入口
│       ├── src/
│       │   ├── main.rs
│       │   ├── lib.rs
│       │   └── commands/         # Tauri Commands
│       ├── src-tauri/
│       │   └── tauri.conf.json
│       └── src/                  # 前端 (React)
│           ├── main.tsx
│           ├── App.tsx
│           ├── components/
│           ├── pages/
│           └── stores/
│
├── docs/
│   ├── product-design.md         # 本文档
│   └── README.md
├── .github/
├── LICENSE
└── README.md
```

### 9.2 Crate 依赖关系

```
studio-app (Tauri 入口)
  ├── studio-core (类型/接口)
  ├── studio-import (数据导入)
  │     └── studio-core
  ├── studio-analysis (分析引擎)
  │     └── studio-core
  ├── studio-storage (持久化)
  │     └── studio-core
  └── studio-report (报告生成)
        └── studio-core
```

### 9.3 关键 Rust 依赖

| Crate | 用途 |
|-------|------|
| `tauri` 2.x | 桌面框架 |
| `duckdb` | 本地数据库 |
| `serde` / `serde_json` | 序列化 |
| `tokio` | 异步运行时 |
| `pdfium-render` | PDF 渲染 |
| `pulldown-cmark` | Markdown 解析 |
| `calamine` | DOCX/XLSX 解析 |
| `regex` | 正则表达式 |
| `uuid` | UUID 生成 |
| `chrono` | 时间处理 |
| `thiserror` | 错误处理 |
| `tracing` | 日志 |

---

## 10. 里程碑与路线图

### 10.1 M0：项目脚手架（第 1-2 周）

- [x] 初始化 Rust workspace + Tauri 2.x 项目（`Cargo.toml` workspace + 6 个 crate，
  `crates/studio-app/src-tauri`）
- [x] 前端 React + TypeScript + Tailwind 脚手架（Tailwind v4 走 `@tailwindcss/postcss`）
- [x] DuckDB 集成 + 表结构迁移（`duckdb` bundled，`migration.rs` 已到 v13，迁移幂等）
- [x] 项目 CRUD（创建/打开/删除）——`list_projects` / `create_project` / `delete_project`，
  「打开」是前端导航到分析页，数据全部来自 DuckDB
- [x] 基础 UI 布局（左侧导航 + 右侧内容）——`App.tsx` 的 `Sidebar` + `main`
- [x] CI/CD 配置（GitHub Actions）——`.github/workflows/ci.yml`

### 10.2 M1：数据导入（第 3-4 周）

- [x] TraceView `semantic_result.json` 解析（`studio-import::semantic`：blocks / tokens /
  entities / noun signals；章节 TOC 与领域术语从另外两个文件补）
- [x] TraceView `popo_result.json` 解析（取章节 TOC，文件缺失时按无 TOC 继续）
- [x] 数据写入 DuckDB（`save_document` 在一个事务里写 documents / semantic_blocks / tokens /
  entities / noun_signals，实体先经 registry 取稳定 id，任一步失败整体回滚；带进度事件）
- [ ] 原始文档导入（Markdown / TXT）
  - ⏳ 目前唯一入口是 TraceView 项目文件夹导入；直接读 Markdown / TXT 未做
- [ ] 项目文件管理（自包含文件夹）
  - ⏳ 现状是 `projects.source_path` 原位引用 TraceView 文件夹，正文与实体全部落
    DuckDB（`app_data_dir`/`narrative_studio.duckdb`）；没有把源文件复制进
    `projects/{project_id}/`，development.md §9.4 的目录树是目标形态而不是现状
- [ ] 数据校验与完整性检查
  - ⏳ 导入侧只校验 `project.json` 与 `semantic_result.json` 存在且可解析，
    知识球导出在写盘前跑 `validate()`；文档与实体之间的一致性检查未做

### 10.3 M2：P0 分析维度（第 5-8 周）

- [x] 统计分析：句子统计、可读性评分
- [x] 评估：总体评估、关键建议（规则引擎，5 维评分）
- [x] 叙事分析：叙事弧线、角色档案与共现
- [x] 文本诊断：重复短语、副词/形容词
  - ⏳ 拼写错误：中文纠错需要词典与分词方案，暂缓到 M3
- [x] 分析结果持久化与缓存：按输入内容指纹（SHA-256）复用，文本变化自动失效，
  UI 提供「重新分析」强制重算
- [x] 基础可视化：柱状图、折线图、雷达图（5 维评分，纯 SVG，无第三方图表库）
- [x] 报告生成：Markdown 导出（`reports` 表入库 + 选择路径写盘）
- [x] 情节分析（plot_analysis）：从实体流提取情节线（贯穿 / 断续 / 集中 / 仅铺垫 / 迟到 / 次要）、
  转折点与连贯性评分；UI 提供情节线表、转折点与「需要处理的线索」，报告同步导出。
  LLM 的情节归并属于 M4 的可选云层

### 10.4 M3：P1 分析维度（第 9-12 周）

- [x] 叙事分析：节奏（pacing_analysis）：按章节切段计算张力（0-100 绝对刻度：对话占比 /
  短句比例 / 感叹密度 / 实体密度），输出张力波动、连续平淡区间、拖沓章节与节奏分；
  UI 提供节奏曲线（折线图）、逐段明细与问题清单，报告同步导出
- [x] 叙事分析：冲突（conflict_analysis）：四类冲突词库（人际 / 内心 / 社会 / 生存）按章节切段计数，
  输出强度曲线（0-100 绝对刻度）、类型占比与高频词、峰值位置、结尾三分之一平均与无冲突区间；
  未检测到冲突用词时不评分（而不是给 0）。UI 提供冲突曲线、类型明细与逐段明细，报告同步导出
- [x] 叙事分析：主题（theme_analysis）：从名词信号（缺失时回退到名词词性）收集复现词，按包含关系与
  同段共现合并同类（出版 / 出版业 记为一个主题），排除人名与时间表达，输出主题清单（提及 / 覆盖 /
  占比 / 贯穿度 / 分布）、贯穿曲线（与节奏、冲突共用同一横轴）、单次词占比与主题分；
  没有可用名词词汇或没有任何词复现时不评分（而不是给 0）。UI 提供主题表、贯穿曲线与体量对比，
  报告同步导出。反复出现只说明复现，是否构成母题留给人判断；LLM 的母题归纳属于 M4
- [x] 叙事分析：场景/设定（setting_analysis）：按地点实体（LOCATION / FACILITY）与环境词库把文档切成场景
  （换地点即开新场，未点名的块沿用当前场景，标题块强制开新场），输出场景清单（地点 / 块 / 字数 /
  环境密度 / 对话占比 / 时间标记 / 在场人物）、地点清单（提及 / 覆盖 / 首次末次 / 缺席与回归）、
  环境密度曲线（与节奏、冲突、主题共用同一横轴）与场景问题（无地点锚点 / 地点过密 / 长时间离开地点 /
  环境描写偏少）；既没有点名地点也没有环境用词时不评分（而不是给 0）。地点与描写二者其一即可支撑场景，
  因此只扣「两者都没有」。UI 提供地点表、场景清单、环境曲线与问题清单，报告同步导出
- [x] 文本诊断：陈词滥调（cliche_analysis）：按 121 项短语库（表情动作 / 身体反应 / 情绪直陈 / 套路比喻 /
  场面套话 / 说话套话 / 时间过渡 / 外貌套话）逐字扫描：同一位置只取最长匹配，所以「空气仿佛凝固」只算一次而不会被
  嵌套的短词重复命中；`(淡淡|浅浅)一笑` 这类同义变体归为一个短语并列出实际形式。输出万字命中率与判定、短语清单
  （次数 / 占比 / 覆盖 / 首次末次 / 改写方向）、命中位置（上下文 + 块内字偏移，引号内的命中单独标记为对白）、
  密度曲线（与节奏、冲突、主题、场景共用同一横轴）与用词问题（反复使用 / 局部密集 / 依赖同一句 / 对白标签过多）；
  不足 1000 字时按万字密度评分没有意义，因此不评分（而不是给 0）。UI 提供短语表、密度曲线、命中清单与问题清单，
  报告同步导出。命中只说明词面撞上库里的常见说法，是否真是陈词滥调、要不要改留给人判断；词库之外的表达不会被发现
- [x] 统计分析：对话/叙事比例（dialogue_analysis）：按两种信号识别对白——引号（“ ” 「 」 『 』）内的文字，以及以
  ——开头的行。引号在换行处收口：跨段书写的长对白每段都以「开头、只有最后一段收尾，因此不能让引号跨过中间的叙述；
  已被引号覆盖的破折号行不重复计数。输出对白/叙事字数与占比、判定（未见对白 / 偏少 / 混合 / 以对白为主）、对白处数
  （引号 / 破折号分列）、平均每处与最长一处、长度分布（1-9 / 10-29 / 30-99 / 100-299 / 300+ 固定档位）、最长对白样本
  （原文片段 + 识别方式）、连续对白块与连续无对白区间（块数与字数）、对白占比曲线（与节奏、冲突、主题、场景、
  陈词滥调共用同一横轴，0-100 强度刻度）与比例问题（没有对白 / 长时间无对白 / 局部对白过密 / 长独白 / 对白过短 /
  对白未加引号 / 各段比例不均）；不足 300 字的段不参与最密、最疏与差异比较，否则一个片段里的一个引号就能压过一整章。
  **本维度不给出评分**：30% 对白并不比 5% 更好或更坏，缓存行的 `score` 记为 NULL 而不是 0。UI 提供占比卡片、曲线、
  长度分布、最长对白与问题清单，报告同步导出。识别只依赖标点：原文不用引号时会漏判，「未见对白」不等于没有人说话
- [x] 内容分析：显式语言（explicit_analysis）：按 35 项词库（粗口辱骂 / 性描写 / 性暴力 / 暴力血腥 / 自伤 /
  贬损歧视 / 毒品）与三档用词强度（轻 / 中 / 重）逐字扫描，与陈词滥调共用同一个匹配器：同一位置只取最长匹配、
  命中不重叠，`(强奸|性侵|轮奸)` 这类同义变体归为一个词条并列出实际命中的形式。输出内容分级（词库未见 / 轻微 /
  中等 / 强烈，取全文最强的那一档）、三档命中数（三档恒常列出，0 也要看起来是测过的而不是漏测的）、类型分布、
  需要读者提示的内容族、词条表（次数 / 占比 / 覆盖 / 首次末次 / 词库说明）、命中位置（上下文 + 块内字偏移，
  引号内的命中单独标记为对白）、密度曲线（与节奏、冲突、主题、场景、陈词滥调、对话共用同一横轴）与内容提示
  （自伤描写 / 性暴力 / 露骨性描写 / 血腥细节 / 贬损歧视用语 / 毒品描写 / 局部密集）；自伤族在中等强度即提示，
  因为这一族一次提及比密度更值得注意，而「打架」这类普通的轻度暴力单独出现不构成提示。**本维度不给出评分**：
  写了什么不等于写得好不好，一部战争题材并不比田园题材更差，缓存行的 `score` 记为 NULL 而不是 0。UI 提供分级徽章、
  三档计数、类型分布、词条表、密度曲线、命中清单与提示清单，报告同步导出。命中只说明词面撞上词库，不等于剧情如此；
  分级只反映用词强度，不反映事件本身；「词库未见」只说明这 35 个模式没有命中，不代表内容温和，是否需要读者提示由人决定
- [x] 故事结构分析（structure_analysis）：复用节奏的张力曲线与情节线的线程变化，先在文档里找出自己的地标——开篇、
  线程转折（某段里线索新出现或消失）、张力高点 / 低点 / 全文最高点（与均值相差 1.5 倍标准差以上才算），再按位置与
  三个经典模型对照：三幕结构（铺垫 / 第一转折点 / 中点 / 第二转折点 / 高潮 / 收束）、起承转合（起 / 承 / 转 / 合）、
  英雄之旅（日常世界 / 冒险的召唤 / 跨越门槛 / 试炼与伙伴 / 最深洞穴 / 复活 / 携宝归来，只取其中可用信号定位的七环）。
  每一环只有三种结果：命中（区间内有地标）、错位（最近的地标在区间外，给出偏差）、未见（区间内没有任何信号）。
  同时按本文自己最强的两个转折点切成三幕，给出每幕的字数、占比与张力（低 / 均 / 高），并列出结构提示（开头无变化 /
  中段空白 / 张力最高点偏早 / 结尾未回落 / 幕长失衡 / 地标太少）。**本维度不给出评分**：契合三幕不是优点，有意拒绝
  这个形状也不是缺点，缓存行的 `score` 记为 NULL 而不是 0。UI 提供地标位置条、地标表、幕表、每个模型的契合判定
  （吻合 / 部分吻合 / 信号不足，恒与命中 / 错位 / 未见的计数同处一行）与节拍落点表，报告同步导出。地标来自线程变化与
  张力曲线，两者都是代理信号；「未见」只说明这些信号没有显示这一环，不等于故事缺少它；节拍区间（如 25%/50%/25%）是
  写作手册里的约定，不是测量；章节不足 3 个时曲线落在按字数等分的块上，位置是算术结果而不是作者自己的分段；可见正文
  不足 1500 字或没有张力曲线时不做对照，并说明原因
- [x] 可视化：关系图（角色关系环形图，纯 SVG）：节点是提及最多的人物实体（默认 12 个），
  边是两个名字出现在同一个块里；节点大小按提及次数开方缩放（一个角色被提 40 次要比 10 次显眼，
  但不能大到吞掉整张图），线宽按共现次数。布局是确定的环：按提及数排在环上，同一份输入每次画出
  同一张图——力导向一类迭代布局每次收敛不同，既不可复现也无法断言。共现只说明两个名字同块出现，
  不说明他们是什么关系；未画出的角色数量在图下注明；几何与建边放在 `src/lib/graph.ts` 单测
- [x] 时间线分析（timeline_analysis）：扫描正文里的时间表达，而不是读取 DATE 实体——NER 只认 `2003年3月12日`，
  看不见 三天后 / 次年春天 / 那天傍晚，而它认得的每一处都会被重复计数。四类标记分开对待：绝对（有年份，能落到日历上）、
  缺年的日期（`3月12日`：真实，但无法与别处的年份比较）、相对（三天后、次日：有距离，锚在叙述当下而不是日历）、
  模糊（春天、傍晚、那天：只有当下的味道）。只有解析到月份以上的日期参与排序——只到年的 `2003年` 后接 `2003年6月`
  不算倒退，也不算前进。排序给出时间倒回（后文日期早于前文，并说明倒叙与笔误分不出）、时间跨度与大跳跃；扫描另列出
  连续三段以上没有任何标记的区间。识别范围：阿拉伯数字与逐字写出的中文年份、`2003-03-12`、天/周/月/年 的相对表达、
  季节与时段词、节气与节日、星期，以及另外三种历法写法：年号纪年（`乾隆三年` → 1738，只到年，因为改元落在年中）、
  民国纪年（`民国三十八年` → 1949，其月日按公历）、干支纪年（`庚子年`：六十年一轮回，只有本文自己给出的年份能唯一
  框定时才定年，否则记为未定年）与农历月日（`腊月初八`、`乾隆三年三月初五`：先换算成公历，才与其余日期排在同一条线上）。
  年号表取自公开的历代年号对照（CC BY-SA 4.0，据李崇智《中国历代年号考》），新莽至民国凡给出年代的年号全部收录（700
  余个、上千行）；农历换算用开源历法引擎 lunar-lite（MIT，
  内核来自 6tail/tyme4rs，走新月与节气天文而非硬编码表，所以 1738 与 2024 一样准）；两处出处在 UI 与报告的边界说明里署名。
  外文日期仍不在范围。宁可拒绝也不猜：两位数年份（`98年`）、`2月30日`、`六日`
  这类数量、`三年` 这类时长都不算日期；年号名只有紧跟在位年数与「年」时才读作日期（`正统的观点` 不算，`正统十四年` 算），
  在位年数超出该年号的跨度即拒绝，同一年号被两位皇帝用过且两段都对得上时只计数不定年（`至元三年` 是 1266 还是 1337，
  `上元二年` 更多解），该年该月没有那天的
  农历日期（`乾隆三年十二月三十`）一律拒绝而不修补；节气、节日与星期词只按字面识别，同样的字也可能是人名或书名
  （《清明上河图》不是清明，`周三多` 可能是作者），`周` 还是朝代名——`西周末期` 不读作周末。
  月按 30 天、年按 365 天估算，而相对标记从不进入排序，所以这个近似污染不了任何日期。
  **本维度不给出评分**：不写日期的故事不比写满日期的故事差，缓存行的 `score` 记为 NULL 而不是 0。扫描同时数一遍导入里的
  DATE 实体（并不读取它们）：两个数字相差多少，说明有多少日期是用本文不认的写法写的——脱敏后的测试样本正是这种情形
  （实体 19 处、可读 0 处，因为 CJK 被 1:1 替换而数字保留），报告因此说「读不到」而不是「没有日期」。UI 提供标记位置条、
  写法分布（年号/干支/农历/节气等写法各多少处，只在文本真的用了时出现）、日期排序表、倒回与跳跃表、无标记区间、标记样本与提示清单，报告同步导出；可见正文不足 600 字时不做排序并说明原因
- [x] PDF 导出：以打印就绪 HTML 落地。`render_html` 是纯函数，把**已入库的那份报告 Markdown**
  转成自包含单文件（`@page` A4、CJK 字体栈、跨页重复表头、表格行不跨页，屏幕上提示按 ⌘P），
  分析页「导出 PDF」选路径后由 Rust 侧写盘并记录。它不是第二套渲染器：报告的措辞只有一处，
  HTML 只是那份文本的另一种排版，因此导出与 Markdown 不可能给出不同的数字，也不需要把 HTML 再存一遍。
  真正一键产出 `.pdf` 要引入内嵌 CJK 字体的排版引擎（体积、构建时间与字体授权都要另议），留给 M4

### 10.5 M4：增强与打磨（第 13-16 周）

- [ ] LLM 增强分析（可选云层）
- [ ] 作者声音分析
- [ ] 最终审查清单
- [ ] 暗色模式
- [ ] 性能优化
- [ ] 用户文档
- [ ] Beta 发布

---

## 11. 非功能性需求

### 11.1 性能

| 指标 | 目标 |
|------|------|
| 应用启动时间 | < 3 秒 |
| TraceView 结果导入（1000 块） | < 5 秒 |
| P0 维度分析（5 万字文档） | < 30 秒 |
| 报告生成 | < 5 秒 |
| 内存占用（空闲） | < 200 MB |
| 内存占用（分析中） | < 500 MB |

### 11.2 可靠性

- 所有分析操作幂等（可重复执行）
- DuckDB 事务保证数据一致性
- 分析失败不影响已有数据
- 自动备份项目文件

### 11.3 安全性

- 所有数据本地存储，不上传（除非用户主动启用云功能）
- LLM API 密钥本地加密存储
- 项目文件权限控制

### 11.4 可维护性

- 遵循 NarrativeOS 工程规则（9 条强制规则）
- 每个 crate 独立可测试
- 分析维度可插拔（插件 API 合约）
- 完整的类型化接口与文档

### 11.5 跨平台

| 平台 | 支持状态 |
|------|---------|
| macOS (Apple Silicon) | M0 起支持 |
| macOS (Intel) | M0 起支持 |
| Windows 10/11 | M0 起支持 |
| Linux (x86_64) | M1 起支持 |

---

## 附录 A：截图原始内容（英文）

### A.1 左侧导航栏

```
My Marlowe Report
  Get started
  Overall assessment
  Points of concern
  Narrative arc analysis
  Plot analysis
  Story elements
  Character analysis
  Pacing analysis
  Conflict analysis
  Theme analysis
  Setting analysis
  Author voice
  Dialogue/narrative
  Readability
  Clichés finder
  Explicit language
  Repetitive phrases
  Adverbs/adjectives
  Misspellings finder
  Key recommendations
  Story structure
  Review & revise
  Feedback
  Next steps
```

### A.2 主内容区

**Title:** Points of concern

**Overview:**
When you're ready to upload a work, our AI will analyze the story and spotlight areas of the manuscript that could be refined to increase clarity, resolve inconsistencies, and avoid scenes that seem forced or out of place. It will also review the work for any potential issues that might warrant a trigger warning or reader advisory.

Subscribe to **Marlowe Pro** to see the points of concern identified in Marlowe's analysis of your manuscript.

[GO PRO]

### A.3 示例报告 (SAMPLE ONLY)

**Title:** Points of concern
**Tags:** Debut | Experienced

**📌 Possible areas for improvement and potential issues**

In this section, we'll spotlight areas of your work that could be refined to increase clarity, resolve inconsistencies, and avoid sequences that seem forced or out of place.

1. **Dan's transformation from helpful journalist to killer feels too abrupt**—his obsession with Tobey needs more gradual development throughout the story to make his revelation feel earned rather than shocking purely for plot purposes.

2. **Tobey's complete unawareness of Ty's emotional distance seems unrealistic**—for a man who loves his wife and works closely with her daily, his failure to notice her growing detachment from their marriage strains credibility.

3. **The timing of Chase's trade to the Yankees feels too convenient**—while it serves the plot, the confluence of events that brings him back precisely when the team needs him and when Ty is most vulnerable seems orchestrated rather than organic.

4. **Ty's professional competence versus personal confusion creates inconsistency**—she demonstrates sophisticated business judgment and team management skills but makes repeatedly poor personal decisions that seem out of character for someone so analytical.

5. **The serial killer plot connection to the romance feels forced**—while both plots involve themes of obsession and loyalty, the way Dan's murders specifically target Tobey's affairs stretches credibility and makes the thriller elements feel grafted onto the romance rather than naturally integrated.

---

## 附录 B：NarrativeOS 工程规则合规检查

| 规则 | 合规方式 |
|------|---------|
| #1 运行时隔离 | Rust 后端 + WebView 前端，Tauri IPC 隔离 |
| #2 禁止跨运行时导入 | 前端不直接访问 Rust 类型，通过 Tauri Commands |
| #3 DuckDB 规范化存储 | 所有持久化数据存于 DuckDB |
| #4 IPC 优先 | Tauri Commands 作为唯一跨运行时通道 |
| #5 插件 API 合约 | 分析维度通过 trait 接口注册 |
| #6 云可选 | LLM 增强为可选层，本地规则引擎独立可用 |
| #7 Rust workspace crates | 5 个独立 crate + workspace root |
| #8 类型化接口 | 所有公共 API 使用 Rust 类型 + serde 契约 |
| #9 文档同步 | 代码变更同步更新本文档 |