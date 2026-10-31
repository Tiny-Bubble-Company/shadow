---
title: "How to protect customer data in RAG pipelines sent to LLMs"
seoTitle: "How to protect customer data in RAG pipelines sent to LLMs"
description: "Keep customer names, emails and IDs in retrieved chunks away from OpenAI or Anthropic. Mask RAG context at generation time with Shadow and restore the answer."
keywords:
  - rag pii masking
  - protect customer data rag
  - anonymize retrieval augmented generation
  - pii in vector database llm
  - mask context before llm
  - secure rag pipeline
faq:
  - q: "Where does customer data leave a RAG pipeline?"
    a: "At two points: when chunks are sent to a hosted embedding API, and when retrieved chunks are pasted into the generation prompt. If your vector store and retriever run in your own infrastructure, the generation call is usually the larger exposure, and it is where Shadow fits most naturally."
  - q: "Should I embed documents through Shadow?"
    a: "Only if sending real text to the embedding provider is unacceptable. Shadow masks JSON embedding requests to intercepted hosts such as OpenAI and Voyage AI, so the index then holds embeddings of fakes. That works consistently with a persistent vault, but it ties the index to that vault and can shift retrieval quality. Embedding locally with an open model is often simpler."
  - q: "Will masking hurt answer quality in RAG?"
    a: "Usually not noticeably for content questions, because fakes keep the same shape and are consistent across chunks. For bulk context, the report intent swaps people, emails and phones for typed tokens so the model carries them through without inventing details."
  - q: "Does Shadow work with LangChain and LlamaIndex?"
    a: "Yes. Both accept a base URL for their OpenAI or Anthropic clients. Point it at the Shadow gateway, or run the pipeline with HTTPS_PROXY set in proxy mode."
  - q: "Are citations and source IDs preserved?"
    a: "Document IDs and URLs on public domains are left alone. Internal hostnames, emails and names inside chunk metadata are masked like any other string and restored in the answer, so citations still point to the real source on your side."
sidebar:
  order: 11
---

To protect customer data in a RAG pipeline, keep the document store and retrieval inside your infrastructure and mask the retrieved chunks at the moment they are sent to the model. Shadow does that masking in transit: names, emails, IBANs, hostnames and secrets in the prompt and its context become reversible fakes, and the model's answer, including streamed text and citations, comes back with real values.

## Where does customer data leak in a RAG pipeline?

| Stage | What leaves | Typical destination |
|---|---|---|
| Ingestion: chunk embedding | Every chunk of every document | Hosted embedding API (OpenAI, Voyage AI, Cohere) |
| Query embedding | The user's question | Same embedding API |
| Generation | The question plus the top *k* retrieved chunks, often with metadata | Chat model API |
| Observability | Prompts and completions | Tracing tools, gateway logs, caches |

Retrieved chunks are the dangerous part. They are selected *because* they are relevant to a specific customer, so they concentrate exactly the data you would least like to send: the support history, contract or account notes for that person.

## Recommended architecture

```text
              your infrastructure                                  provider
┌──────────────────────────────────────────────────────┐
│ documents ──▶ embed ──▶ vector store (real text)     │
│ question  ──▶ retrieve top-k ──▶ build prompt        │
│                                     │                │
│                               Shadow (mask) ─────────┼──▶ chat model
│ answer   ◀── Shadow (restore) ◀──────────────────────┼─── response
└──────────────────────────────────────────────────────┘
```

1. **Store real data locally.** Your vector store holds real text, so retrieval, filtering and access control work on real values.
2. **Embed where you can control it.** Use a local embedding model, or a hosted one covered by your agreements.
3. **Mask at generation.** Route the chat call through Shadow, so the question and the retrieved context are masked together with consistent fakes.
4. **Restore on the way back.** Answers and citations refer to real customers on your side only.

## Step by step

### 1. Run Shadow next to the RAG service

```sh
shadow init --org "Acme GmbH"
shadow run
```

In Kubernetes, run it as a [sidecar](/shadow/integrate/docker-kubernetes/) with a persistent `/data` volume.

### 2. Point the generation client at the gateway

```python
from langchain_openai import ChatOpenAI

llm = ChatOpenAI(model="gpt-4o-mini", base_url="http://127.0.0.1:8877/gw/openai/v1")
```

```python
from llama_index.llms.anthropic import Anthropic

llm = Anthropic(model="claude-sonnet-4-5", base_url="http://127.0.0.1:8877/gw/anthropic")
```

Or set `OPENAI_BASE_URL` / `ANTHROPIC_BASE_URL` for the whole process.

### 3. Add the vocabulary in your corpus

Customer company names, product codenames and internal domains are what retrieval surfaces most:

```toml
organization = ["Acme GmbH"]
terms = ["org:Globex Ltd", "org:Initech", "Project Falcon"]
internal_domains = ["acme.example", "kb.acme.internal"]
```

For corpora full of free-text names, enable the open-weight [NER sidecar](/shadow/concepts/detection/#local-ner-sidecar) (OpenAI Privacy Filter and GLiNER) or [Presidio](/shadow/concepts/detection/#presidio).

### 4. Use tokens for bulk context

When a request carries many rows (a CRM export, many tickets) or asks for a report, Shadow's [intent policy](/shadow/concepts/intent-policy/) classifies it as `report` and uses typed tokens for people and contact details:

```toml
[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"
```

### 5. Check what the chunks contain

Export a sample of chunks to a text file and scan it:

```sh
shadow scan sample-chunks.txt
shadow scan sample-chunks.txt --json > findings.json
```

## Example

User question and retrieved context, as built by the app (fictional):

```text
Q: Does our policy cover water damage for Tom Becker at 12 Elm Road?
Context: [chunk 1] Tom Becker, policy holder, tom.becker@mail.example
```

What the model receives (output from `shadow mask` on one machine; yours will differ):

```text
Q: Does our policy cover water damage for Qiang Xu at 211 Jianguo Road?
Context: [chunk 1] Qiang Xu, policy holder, jing.zhu@bryula.cn
```

The person, address and email are consistent between the question and the chunk, so the model can connect them. The answer comes back about Tom Becker at 12 Elm Road: each full fake is restored, and so is a fake first or last name on its own. "Mr Xu" is the exception: name parts under three characters are not restored, nor are other fragments such as part of the address, so ask the model to use full names and addresses in answers.

## What about embeddings?

Shadow intercepts embedding hosts such as `api.openai.com` and `api.voyageai.com`, and embedding requests are JSON, so text sent to them through Shadow is masked too. If you embed through Shadow:

- the index stores embeddings of **fakes**, and query embeddings go through the same vault, so a customer's name maps to the same fake at indexing and at query time;
- the index is then tied to that vault: losing or resetting `vault.bin` and `vault.key` changes future fakes, and stored vectors stop matching new queries on those names;
- semantic similarity on names and companies is now similarity between fakes, which can shift ranking.

That is a reasonable trade if the embedding provider must never see real text. Otherwise, embed locally and keep Shadow on the generation path.

## Limitations

- Shadow does not filter what is retrieved: access control on documents is still your retriever's job.
- Scanned PDFs and images in the corpus are not OCR'd.
- Internal record numbers (claim, matter, account references) are not detected by the built-in detectors; there is no custom regex setting in `shadow.toml`.
- Context can identify: a chunk describing a unique incident can point to a customer without any name.

## Related

- [File masking](/shadow/concepts/file-masking/) and [redacting documents before upload](/shadow/use-cases/redact-documents-before-uploading-to-ai/)
- [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/)
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/)
- [SaaS and technology](/shadow/industries/saas-technology/), [insurance](/shadow/industries/insurance/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
