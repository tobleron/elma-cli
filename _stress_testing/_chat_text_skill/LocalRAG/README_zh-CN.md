# Local RAG

一个从零构建的轻量级 RAG（检索增强生成）系统，专为学习目的设计。

[![GitHub stars](https://img.shields.io/github/stars/YannBuf/LocalRAG)](https://github.com/YannBuf/LocalRAG/stargazers)
[![Python](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/)
[![Streamlit](https://img.shields.io/badge/streamlit-1.40+-red.svg)](https://streamlit.io/)

## 简介

本项目实现了一个完整的 RAG 流程，基于 OpenAI 兼容 API，提供功能完整的 Streamlit Web UI：

```
文档 → 加载 → 分块 → 向量化 → 向量存储 → 检索 → LLM → 回答
```

### 核心特性

- **从零实现** — 不依赖 LangChain/LlamaIndex，吃透每个组件原理
- **轻量快速** — 使用 Chroma 向量数据库，最小化依赖
- **兼容任意 API** — 支持 LM Studio、Ollama、vLLM 等 OpenAI 兼容接口
- **5 标签页 Web UI** — 配置、分块、文档、RAG、可观测性
- **5 种分块策略** — 固定大小、递归、结构、语义、LLM 驱动
- **对话历史** — JSON 持久化，自动保存聊天记录
- **用户反馈** — 对每个回答点赞/点踩，持久化存储
- **文档管理** — 查看、筛选、删除已索引文档
- **可观测性** — 结构化日志、Prometheus 指标、日志查看器
- **混合检索** — BM25 + 向量相似度，支持权重配置
- **重排序** — 支持 API / 本地 / HuggingFace 三种 CrossEncoder 模式
- **MMR** — 最大边际相关性，获取多样化检索结果
- **增量 Upsert** — 仅重索引有变化的块，复用已有向量

## 技术栈

| 组件 | 技术 |
|------|------|
| LLM | OpenAI 兼容 API |
| 向量嵌入 | OpenAI 兼容 API |
| 向量数据库 | Chroma |
| Web UI | Streamlit |
| 日志 | structlog + RotatingFileHandler |
| 指标 | prometheus-client |
| 测试 | pytest |

## 项目结构

```
SimpleRag/
├── config/
│   └── api_settings.yaml     # API 配置（LLM、嵌入、重排序）
├── data/
│   ├── chroma_db/            # Chroma 向量数据库
│   ├── chat_history.json     # 对话历史
│   ├── feedback.json         # 用户反馈
│   └── uploads/              # 上传的文档
├── logs/
│   └── app.log               # 应用日志（自动轮转）
├── src/
│   ├── __init__.py
│   ├── loader.py             # 文档加载器（txt、md、pdf）
│   ├── chunker.py            # 分块器封装（兼容旧版）
│   ├── chunkers/             # 分块策略包
│   │   ├── __init__.py
│   │   ├── base.py           # 抽象基类
│   │   ├── _registry.py      # 分块器注册表
│   │   ├── fixed_size_chunker.py
│   │   ├── recursive_chunker.py
│   │   ├── structure_chunker.py
│   │   ├── semantic_chunker.py
│   │   └── llm_chunker.py
│   ├── embedder_api.py       # 嵌入 API 客户端（重试、缓存、批处理）
│   ├── vectorstore.py        # Chroma 封装，支持 upsert 和 HNSW 配置
│   ├── retriever.py          # 混合检索、MMR、重排序、缓存
│   ├── history_manager.py    # 对话历史和反馈持久化
│   ├── llm_api.py            # LLM API 客户端（重试、流式）
│   ├── pipeline.py           # RAG 流程编排
│   ├── observability.py      # 日志、指标、追踪
│   └── app.py                # Streamlit 应用（5 标签页）
├── tests/
│   ├── test_loader.py
│   ├── test_chunker.py
│   ├── test_pipeline.py
│   ├── test_history_manager.py
│   └── test_chunkers/
├── CHANGELOG.md
└── README.md / README_zh-CN.md
```

## 快速开始

### 1. 安装依赖

```bash
pip install -r requirements.txt
```

### 2. 启动 OpenAI 兼容 API 服务

**LM Studio**（本地推荐）：
1. 下载 [LM Studio](https://lmstudio.ai/)
2. 下载一个模型（如 Llama 3.2）
3. 点击 "Start Server" — 默认地址 `http://localhost:1234/v1`

**Ollama**：
```bash
ollama serve
# 默认地址：http://localhost:11434/v1
```

### 3. 运行

```bash
streamlit run src/app.py --server.port 8501
```

在浏览器中打开 `http://localhost:8501`。

### 4. 配置

在 **配置（Configuration）标签页** 中设置 API 地址和模型名称，然后点击 **应用配置（Apply Configuration）**。

## 界面说明

### 5 个标签页

| 标签页 | 功能 |
|--------|------|
| **⚙️ Configuration** | API 端点配置（LLM、嵌入、重排序）、检索参数 |
| **🔪 Chunking** | 上传文档、选择分块策略、预览并索引 |
| **📁 Documents** | 查看已索引文档、块数量、按文档或全部删除 |
| **💬 RAG** | 与文档对话，侧边栏显示对话历史 |
| **📊 Observability** | 实时日志查看、Prometheus 指标端点、实时统计 |

### 对话历史与反馈

- 对话自动保存到 `data/chat_history.json`
- 点击侧边栏中的历史对话可重新加载
- 每个回答下方有 👍/👎 按钮，反馈保存到 `data/feedback.json`

### 重排序配置

在 **配置标签页** 中设置：

| 模式 | 配置方式 |
|------|---------|
| **API 模式** | 设置 `重排序 API Base` + `重排序 API Key` |
| **本地模式** | 将 `重排序模型` 设置为本地目录路径 |
| **HuggingFace 模式** | 将 `重排序模型` 设置为 HuggingFace 模型 ID |
| **禁用** | 留空所有重排序字段，使用嵌入相似度兜底 |

## 分块策略

| 策略 | 说明 |
|------|------|
| **固定大小（Fixed Size）** | 按字符数均匀分块，带重叠区域 |
| **递归（Recursive）** | 按分隔符递归拆分（段落→换行→句子→词） |
| **结构（Structure）** | 标题 + 内容作为一个完整语义单元 |
| **语义（Semantic）** | 按句子切分 → 向量化 → 按相似度合并 |
| **LLM 驱动（LLM-based）** | 由 LLM 判断语义完整的断点，最精确但成本高 |

## 测试

```bash
# 运行所有测试
pytest tests/ -v

# 分块器专项测试
pytest tests/test_chunkers/ -v

# 历史管理器专项测试
pytest tests/test_history_manager.py -v
```

## 环境要求

- Python 3.10+
- OpenAI 兼容 API 服务（LM Studio、Ollama、vLLM 等）
- 可选：CrossEncoder 模型（用于重排序）

## 开源协议

MIT
