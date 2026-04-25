# LocalRAG 教程：从零构建一个 RAG 系统

本教程详细讲解如何从零构建一个完整的 RAG（检索增强生成）系统，**不依赖 LangChain/LlamaIndex**，每个组件都有代码解释。

## 教程目录

| 章节 | 文件 | 内容 |
|------|------|------|
| 01 | [RAG 概述](01_RAG概述.md) | RAG 原理、核心流程、项目架构 |
| 02 | [文档加载](02_文档加载.md) | loader.py：PDF / TXT / MD 读取、编码兜底 |
| 03 | [分块策略](03_分块策略.md) | 5 种分块器：固定/递归/结构/语义/LLM |
| 04 | [向量化](04_向量化.md) | embedder_api.py：API + 缓存 + 批处理 + 重试 |
| 05 | [向量存储](05_向量存储.md) | vectorstore.py：Chroma、HNSW、增量 Upsert |
| 06 | [检索原理](06_检索原理.md) | retriever.py：BM25、RRF、MMR、CrossEncoder |
| 07 | [生成阶段](07_生成阶段.md) | llm_api.py：Prompt 构建、流式输出、重试 |
| 08 | [UI 实现](08_UI实现.md) | app.py：Streamlit、session_state、对话历史 |
| 09 | [可观测性](09_可观测性.md) | observability.py：structlog、Prometheus、追踪 |
| 10 | [部署运维](10_部署与运维.md) | Systemd、Docker、Nginx、日志轮转 |

## 阅读建议

**适合读者：**
- 了解 Python 基础
- 对 LLM / RAG 有基本认知
- 想深入理解 RAG 底层原理

**阅读顺序：**
建议按顺序阅读，第 2-6 章是核心：加载 → 分块 → 向量化 → 存储 → 检索。

## 代码对应

每个教程章节对应 `src/` 中的实际代码：

```
教程                    源码文件
───────────────────────────────────────────────
第二章 文档加载    →   src/loader.py
第三章 分块策略   →   src/chunkers/
第四章 向量化      →   src/embedder_api.py
第五章 向量存储    →   src/vectorstore.py
第六章 检索原理    →   src/retriever.py
第七章 生成阶段    →   src/llm_api.py
第八章 UI 实现     →   src/app.py
第九章 可观测性    →   src/observability.py
```

## 环境准备

```bash
# 克隆项目
git clone https://github.com/YannBuf/LocalRAG.git
cd LocalRAG

# 安装依赖
pip install -r requirements.txt

# 启动 LLM 服务（LM Studio 或 Ollama）
# 然后运行
streamlit run src/app.py
```

## 教程特色

- **由简入深**：从最简单的 loader 讲到最复杂的 reranker
- **有代码**：每个组件都有可直接运行的 Python 代码
- **有思考题**：每章结尾的"问题自测"帮助巩固理解
- **实战导向**：代码来自真实项目，可以直接运行

## 参与改进

发现问题？欢迎提交 Issue 或 Pull Request！
