---
layout: doc
title: "Knowledge Base"
lastUpdated: true
---

# Knowledge Base

Ruri includes a RAG (Retrieval-Augmented Generation) based knowledge base system that allows the AI agent to reference and search through your documents during conversations.

## Overview

The knowledge base system works by:

1. **Ingesting** documents in various formats
2. **Chunking** documents into manageable segments
3. **Embedding** chunks using an embedding model
4. **Storing** embeddings in a vector store
5. **Searching** relevant chunks when a query is made
6. **Reranking** results for improved relevance

This enables the AI agent to answer questions based on your specific documents and data, rather than relying solely on its training data.

## Supported Document Formats

| Format   | Extensions        | Description                    |
| -------- | ----------------- | ------------------------------ |
| PDF      | `.pdf`            | Adobe PDF documents            |
| Excel    | `.xls`, `.xlsx`   | Microsoft Excel spreadsheets   |
| DOCX     | `.docx`           | Microsoft Word documents       |
| Plain Text | `.txt`, `.md`   | Plain text and Markdown files  |

## Configuration

### Embedding Model

The embedding model converts text into vector representations for semantic search. You need to configure:

- The embedding model provider and endpoint
- The model identifier
- The API key (if required)
- The embedding dimensions

### Rerank Model

The rerank model improves search results by re-scoring and reordering the initially retrieved chunks. This is an optional but recommended step for better accuracy.

**Configuration fields:**

| Field        | Description                                    |
| ------------ | ---------------------------------------------- |
| Provider     | The rerank model provider                      |
| API URL      | The endpoint for the rerank service            |
| API Key      | Authentication key                             |
| Model        | The rerank model identifier                    |
| Top-K        | Number of top results to return after reranking |

### Chunking Configuration

Documents are split into chunks before embedding. You can configure:

| Parameter        | Description                                    |
| ---------------- | ---------------------------------------------- |
| Chunk size       | Maximum number of tokens per chunk             |
| Chunk overlap    | Number of overlapping tokens between chunks    |
| Separator        | Text separator used for splitting              |

## Pipeline

The knowledge base pipeline processes documents through the following stages:

```
Document → Parse → Chunk → Embed → Store
                                        ↓
Query → Embed Query → Vector Search → Rerank → Context
```

### 1. Document Parsing

Documents are parsed from their original format into plain text. Each format has a dedicated parser:

- **PDF**: Extracts text content from PDF pages
- **Excel**: Reads cell data from spreadsheet sheets
- **DOCX**: Extracts text and formatting from Word documents
- **Plain Text**: Reads content directly

### 2. Chunking

The parsed text is split into overlapping chunks. Chunking parameters control the size and overlap of these segments, balancing between granularity and context preservation.

### 3. Embedding

Each chunk is processed through the embedding model to produce a vector representation. These vectors capture the semantic meaning of the text, enabling similarity-based search.

### 4. Storage

Embeddings are stored in a vector database alongside the original text chunks and metadata.

### 5. Search

When a query is received:

1. The query is embedded using the same embedding model
2. Vector similarity search finds the most relevant chunks
3. Results are optionally reranked using the rerank model
4. The top-ranked chunks are provided as context to the AI model

## Managing Knowledge Bases

### Via Web UI

1. Navigate to the **Knowledge Base** page
2. Create a new knowledge base with a name and description
3. Upload documents to the knowledge base
4. Monitor the ingestion progress
5. Activate or deactivate knowledge bases

### Activating a Knowledge Base

A knowledge base must be activated to be used in conversations. You can activate knowledge bases through:

- The Web UI
- [Config Profiles](/config-profiles) — each profile specifies which knowledge bases are active

When a knowledge base is active, the AI agent will automatically search it when relevant queries are made.

## Performance Tips

- **Use a quality embedding model** — Better embeddings lead to more accurate search results
- **Enable reranking** — Reranking significantly improves result relevance
- **Tune chunk size** — Smaller chunks are more precise but may lose context; larger chunks preserve context but may introduce noise
- **Use overlap** — Overlapping chunks help preserve information at chunk boundaries
- **Keep knowledge bases focused** — A knowledge base for a specific domain performs better than a general-purpose one
