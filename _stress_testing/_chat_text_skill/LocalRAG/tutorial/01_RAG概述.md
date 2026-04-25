# 第一章：RAG 概述

## 什么是 RAG？

**RAG = Retrieval Augmented Generation（检索增强生成）**。

它是一种让大语言模型（LLM）回答问题时"先查资料再回答"的技术架构。LLM 本身不知道你私人文档里的内容，但 RAG 可以先把相关文档片段找出来，连同问题一起发给 LLM，从而给出准确答案。

```
用户提问
   ↓
检索相关文档片段
   ↓
将「问题 + 文档片段」组合成完整 prompt
   ↓
LLM 生成回答
   ↓
返回答案（附带参考来源）
```

## 为什么不用 LangChain/LlamaIndex？

市面上的 RAG 教程大量依赖 LangChain 或 LlamaIndex，这些框架把复杂性封装好了，调用两三行就完成了。但代价是：

- **你不知道底层发生了什么** — 遇到问题很难排查
- **性能问题难以定位** — 是检索慢还是生成慢？
- **难以定制** — 想换一个 Embedding 模型或改分块策略，改动很大

本教程的特点是**从零实现每个组件**，不依赖任何高级框架。你会清楚地理解 RAG 每一个环节的原理和代码。

## RAG 的核心流程

```
文档 (PDF/TXT/MD)
    ↓
[1] 加载 (Loader)     →  原始文本字符串
    ↓
[2] 分块 (Chunker)   →  切成小块，每块含文本+元信息
    ↓
[3] 向量化 (Embedder) →  每块变成一个向量（数字列表）
    ↓
[4] 存储 (VectorStore) →  向量入库（Chroma）
    ↓
[--- 以上是「建库阶段」，离线完成一次 ---]

[--- 以下是「查询阶段」，用户每次提问触发 ---]

用户提问
    ↓
[5] 检索 (Retriever)  →  找到最相关的 K 个文档块
    ↓
[6] 生成 (LLM)        →  把问题+文档块发给 LLM，生成回答
    ↓
返回答案
```

## 我们的项目架构

```
LocalRAG/
├── src/
│   ├── loader.py           # 文档加载
│   ├── chunkers/           # 5 种分块策略
│   ├── embedder_api.py     # 向量化 API 客户端
│   ├── vectorstore.py      # Chroma 向量数据库封装
│   ├── retriever.py        # 检索：向量+BM25+混合+MMR+重排序
│   ├── llm_api.py          # LLM API 客户端
│   ├── pipeline.py          # RAG 流程编排
│   └── app.py              # Streamlit Web UI
├── config/
│   └── api_settings.yaml   # API 配置（API 地址、模型名）
├── data/
│   └── chroma_db/          # 向量数据库文件
└── tutorial/               # 本教程
```

## 硬件要求

- **LLM**：建议至少 4GB 显存（Qwen2.5-3B 等小模型可在 CPU 运行）
- **Embedding**：轻量模型，CPU 即可
- **存储**：向量很占空间，1000 个文档块约 ~10MB
- **推荐**：LM Studio 本地运行 LLM，避免 API 费用

## 下一章预告

下一章我们从最简单的一步开始：**如何把 PDF、TXT、Markdown 文件读入 Python**。

---

**问题自测：**

1. RAG 的全称是什么？
2. RAG 分为哪两个阶段？分别离线/在线？
3. 为什么我们选择不依赖 LangChain？
