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
  ▼
结构化诊断报告（Markdown / PDF / JSON）
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
| 关系图 | 角色关系、情节线 | 力导向图 (D3.js / 自定义 SVG) |
| 柱状图 | 词频、实体分布 | SVG 渲染 |
| 时间线 | 时间线一致性 | 自定义 SVG |
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

- [ ] 初始化 Rust workspace + Tauri 2.x 项目
- [ ] 前端 React + TypeScript + Tailwind 脚手架
- [ ] DuckDB 集成 + 表结构迁移
- [ ] 项目 CRUD（创建/打开/删除）
- [ ] 基础 UI 布局（左侧导航 + 右侧内容）
- [ ] CI/CD 配置（GitHub Actions）

### 10.2 M1：数据导入（第 3-4 周）

- [ ] TraceView `semantic_result.json` 解析
- [ ] TraceView `popo_result.json` 解析
- [ ] 数据写入 DuckDB
- [ ] 原始文档导入（Markdown / TXT）
- [ ] 项目文件管理（自包含文件夹）
- [ ] 数据校验与完整性检查

### 10.3 M2：P0 分析维度（第 5-8 周）

- [ ] 文本诊断：拼写错误、重复短语、副词/形容词
- [ ] 统计分析：句子统计、可读性评分
- [ ] 叙事分析：叙事弧线、情节分析、角色分析
- [ ] 评估：总体评估、关键建议
- [ ] 报告生成：Markdown 导出
- [ ] 基础可视化：雷达图、柱状图

### 10.4 M3：P1 分析维度（第 9-12 周）

- [ ] 叙事分析：节奏、冲突、主题、场景/设定
- [ ] 文本诊断：陈词滥调
- [ ] 内容分析：显式语言
- [ ] 统计分析：对话/叙事比例
- [ ] 故事结构分析
- [ ] 可视化：折线图、关系图、时间线
- [ ] PDF 导出

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