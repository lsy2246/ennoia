const DEFAULT_BASE_URL = "https://api.openai.com/v1";
const DEFAULT_MODEL = "gpt-5.4";
const MODEL_BUDGETS = new Map();

export const provider = {
  id: "openai",
  kind: "openai",
  interfaces: ["generate", "tools", "models"],
  generationOptions: [
    {
      id: "reasoning_effort",
      type: "select",
      values: ["low", "medium", "high"],
      default: "medium",
    },
  ],
  listModels,
  generate,
};

export async function listModels(context = {}) {
  const config = normalizeProviderConfig(context.provider ?? context);
  const response = await openaiFetch(config, "/models", { method: "GET" });
  const data = await response.json();
  const models = Array.isArray(data.data)
    ? data.data
        .map((item) => normalizeModelDescriptor(item?.id))
        .filter(Boolean)
        .sort((left, right) => left.id.localeCompare(right.id))
    : [];

  return {
    provider_id: config.id,
    models,
  };
}

export async function generate(request = {}) {
  const config = normalizeProviderConfig(request.provider ?? {});
  const model = request.model ?? config.default_model ?? DEFAULT_MODEL;
  const generationOptions = normalizeGenerationOptions(request.generation_options ?? request.generationOptions);
  const instructions = normalizeInstructions(request.instructions ?? request.system_prompt);
  const visibleContext = normalizeVisibleContext(request.context ?? request.runtime_context);
  const payload = {
    model,
    messages: toChatCompletionMessages(
      request.messages ?? request.input ?? request.prompt ?? "",
      instructions,
      visibleContext,
    ),
  };
  applyChatCompletionGenerationOptions(payload, generationOptions);

  const tools = normalizeChatCompletionTools(request.tools ?? []);
  if (tools.length > 0) {
    payload.tools = tools;
    payload.tool_choice = request.tool_choice ?? "auto";
  }

  if (request.metadata && typeof request.metadata === "object") {
    payload.metadata = request.metadata;
  }

  if (tools.length === 0) {
    const streamed = await generateByChatCompletionStream(config, model, payload);
    if (streamed.text) {
      return streamed;
    }
  }

  const response = await openaiFetch(config, "/chat/completions", {
    method: "POST",
    body: JSON.stringify(payload),
  });
  const data = await response.json();
  const text = collectChatCompletionText(data);
  const toolCalls = collectChatCompletionToolCalls(data);

  if (!text && toolCalls.length === 0) {
    throw new Error(describeEmptyChatCompletion(data, model));
  }

  return {
    id: data.id,
    model: data.model ?? model,
    text,
    tool_calls: toolCalls,
    raw: data,
  };
}

async function generateByChatCompletionStream(config, fallbackModel, payload) {
  const response = await openaiFetch(config, "/chat/completions", {
    method: "POST",
    body: JSON.stringify({
      ...payload,
      stream: true,
    }),
  });
  const data = await collectChatCompletionStream(response, fallbackModel);
  const text = data.text.trim();
  if (!text) {
    return {
      id: data.id,
      model: data.model ?? fallbackModel,
      text: "",
      tool_calls: [],
      raw: data.raw,
    };
  }
  return {
    id: data.id,
    model: data.model ?? fallbackModel,
    text,
    tool_calls: [],
    raw: data.raw,
  };
}

function normalizeProviderConfig(config) {
  const baseUrl = trimTrailingSlash(config.base_url || DEFAULT_BASE_URL);
  const apiKeyEnv = config.api_key_env || "OPENAI_API_KEY";
  const apiKey = process.env[apiKeyEnv];
  if (!apiKey) {
    throw new Error(`OpenAI API key is missing; set ${apiKeyEnv}`);
  }

  return {
    id: config.id || "openai",
    base_url: baseUrl,
    api_key: apiKey,
    default_model: config.default_model || DEFAULT_MODEL,
  };
}

function normalizeModelDescriptor(value) {
  if (typeof value !== "string") {
    return null;
  }
  const id = value.trim();
  if (!id) {
    return null;
  }
  const budgets = MODEL_BUDGETS.get(id) ?? null;
  return {
    id,
    max_context_tokens: budgets?.max_context_tokens ?? null,
    max_input_tokens: budgets?.max_input_tokens ?? null,
  };
}

async function openaiFetch(config, path, init) {
  const response = await fetch(`${config.base_url}${path}`, {
    ...init,
    headers: {
      authorization: `Bearer ${config.api_key}`,
      "content-type": "application/json",
      ...(init.headers ?? {}),
    },
  });

  if (!response.ok) {
    const body = await response.text();
    const summary = summarizeOpenAiErrorBody(body);
    if (path === "/models" && response.status === 405) {
      throw new Error(
        [
          `当前上游不支持 OpenAI 扩展使用的模型发现接口: GET ${config.base_url}${path}`,
          "这通常说明它不是标准 OpenAI 模型列表接口。",
          "请手动维护模型列表，或者为这个上游提供它自己的模型发现扩展。",
          summary ? `上游返回: ${summary}` : null,
        ]
          .filter(Boolean)
          .join(" "),
      );
    }
    throw new Error(
      summary
        ? `OpenAI request failed: ${response.status} ${summary}`
        : `OpenAI request failed: ${response.status}`,
    );
  }

  return response;
}

async function collectChatCompletionStream(response, fallbackModel) {
  if (!response.body) {
    throw new Error("OpenAI stream response body is missing");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffered = "";
  let text = "";
  let responseId = "";
  let model = fallbackModel ?? "";
  let finishReason = "";
  const events = [];

  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    buffered += decoder.decode(value, { stream: true });
    const parts = buffered.split(/\r?\n\r?\n/);
    buffered = parts.pop() ?? "";
    for (const part of parts) {
      const event = parseSseEvent(part);
      if (!event) {
        continue;
      }
      if (event === "[DONE]") {
        buffered = "";
        break;
      }
      let parsed;
      try {
        parsed = JSON.parse(event);
      } catch {
        continue;
      }
      events.push(parsed);
      if (typeof parsed?.id === "string" && parsed.id.trim()) {
        responseId = parsed.id;
      }
      if (typeof parsed?.model === "string" && parsed.model.trim()) {
        model = parsed.model;
      }
      for (const choice of parsed?.choices ?? []) {
        const delta = choice?.delta;
        if (typeof delta?.content === "string") {
          text += delta.content;
        } else if (Array.isArray(delta?.content)) {
          for (const item of delta.content) {
            if (typeof item?.text === "string") {
              text += item.text;
            } else if (typeof item?.content === "string") {
              text += item.content;
            }
          }
        }
        if (typeof choice?.finish_reason === "string" && choice.finish_reason.trim()) {
          finishReason = choice.finish_reason;
        }
      }
    }
  }

  const flushed = decoder.decode();
  if (flushed) {
    buffered += flushed;
  }

  if (buffered.trim()) {
    const event = parseSseEvent(buffered);
    if (event && event !== "[DONE]") {
      try {
        const parsed = JSON.parse(event);
        events.push(parsed);
        if (typeof parsed?.id === "string" && parsed.id.trim()) {
          responseId = parsed.id;
        }
        if (typeof parsed?.model === "string" && parsed.model.trim()) {
          model = parsed.model;
        }
        for (const choice of parsed?.choices ?? []) {
          const delta = choice?.delta;
          if (typeof delta?.content === "string") {
            text += delta.content;
          }
          if (typeof choice?.finish_reason === "string" && choice.finish_reason.trim()) {
            finishReason = choice.finish_reason;
          }
        }
      } catch {
        // ignore trailing incomplete payload
      }
    }
  }

  return {
    id: responseId || "unknown",
    model: model || fallbackModel || DEFAULT_MODEL,
    text,
    finish_reason: finishReason || "unknown",
    raw: {
      object: "chat.completion.stream",
      id: responseId || "unknown",
      model: model || fallbackModel || DEFAULT_MODEL,
      finish_reason: finishReason || "unknown",
      events,
    },
  };
}

function parseSseEvent(chunk) {
  const lines = chunk
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (lines.length === 0) {
    return "";
  }
  const dataLines = lines
    .filter((line) => line.startsWith("data:"))
    .map((line) => line.slice(5).trim());
  if (dataLines.length === 0) {
    return "";
  }
  return dataLines.join("\n");
}

function toChatCompletionMessages(input, instructions, visibleContext) {
  const messages = [];
  if (instructions.base) {
    messages.push({ role: "system", content: instructions.base });
  }
  if (visibleContext) {
    messages.push({ role: "system", content: visibleContext });
  }
  if (typeof input === "string") {
    messages.push({ role: "user", content: input });
    return messages;
  }
  if (!Array.isArray(input)) {
    messages.push({ role: "user", content: String(input ?? "") });
    return messages;
  }

  return [
    ...messages,
    ...input.map((message) => ({
      role: normalizeRole(message.role ?? message.sender),
      content: normalizeMessageContent(message.content ?? message.body ?? message.text ?? ""),
    })),
  ];
}

function normalizeRole(role) {
  if (role === "agent") {
    return "assistant";
  }
  if (role === "operator") {
    return "user";
  }
  if (role === "assistant" || role === "system" || role === "developer" || role === "tool") {
    return role;
  }
  return "user";
}

function normalizeMessageContent(content) {
  if (typeof content === "string") {
    return content;
  }
  if (!Array.isArray(content)) {
    return String(content ?? "");
  }
  return content
    .map((part) => {
      if (typeof part === "string") {
        return part;
      }
      if (typeof part?.text === "string") {
        return part.text;
      }
      return "";
    })
    .join("\n");
}

function normalizeInstructions(value) {
  if (!value) {
    return {};
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed ? { base: trimmed } : {};
  }
  if (typeof value !== "object" || Array.isArray(value)) {
    const normalized = String(value ?? "").trim();
    return normalized ? { base: normalized } : {};
  }

  const base = typeof value.base === "string" ? value.base.trim() : "";
  return base ? { ...value, base } : {};
}

function normalizeVisibleContext(context) {
  if (context == null) {
    return "";
  }
  if (typeof context === "string") {
    return context.trim();
  }
  if (typeof context !== "object") {
    return String(context).trim();
  }
  try {
    return JSON.stringify(context, null, 2);
  } catch {
    return "";
  }
}

function normalizeChatCompletionTools(tools) {
  return tools.map((tool) => {
    if (tool?.type === "function" && tool.function) {
      return tool;
    }
    return {
      type: "function",
      function: {
        name: tool.name,
        description: tool.description ?? "",
        parameters: tool.parameters ?? tool.input_schema ?? { type: "object", properties: {} },
      },
    };
  });
}

function normalizeGenerationOptions(options) {
  if (!options || typeof options !== "object" || Array.isArray(options)) {
    return {};
  }
  return options;
}

function applyChatCompletionGenerationOptions(payload, options) {
  const reasoningEffort = normalizeReasoningEffort(options.reasoning_effort);
  if (reasoningEffort) {
    payload.reasoning_effort = reasoningEffort;
  }

  applyNumericOption(payload, "temperature", options.temperature);
  applyNumericOption(payload, "top_p", options.top_p);
  applyNumericOption(payload, "presence_penalty", options.presence_penalty);
  applyNumericOption(payload, "frequency_penalty", options.frequency_penalty);
  applyIntegerOption(payload, "max_completion_tokens", options.max_completion_tokens);
}

function normalizeReasoningEffort(value) {
  if (typeof value !== "string") {
    return "";
  }
  const normalized = value.trim().toLowerCase();
  if (!normalized) {
    return "";
  }
  const supportedValues = new Set(["none", "minimal", "low", "medium", "high", "xhigh"]);
  return supportedValues.has(normalized) ? normalized : "";
}

function applyNumericOption(payload, key, value) {
  const normalized = normalizeFiniteNumber(value);
  if (normalized == null) {
    return;
  }
  payload[key] = normalized;
}

function applyIntegerOption(payload, key, value) {
  const normalized = normalizeFiniteNumber(value);
  if (normalized == null) {
    return;
  }
  payload[key] = Math.trunc(normalized);
}

function normalizeFiniteNumber(value) {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) {
      return null;
    }
    const parsed = Number(trimmed);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return null;
}

function collectChatCompletionText(response) {
  const texts = [];

  for (const choice of response.choices ?? []) {
    pushTextCandidate(texts, choice?.message?.content);
    pushTextCandidate(texts, choice?.message?.reasoning_content);
    pushTextCandidate(texts, choice?.message?.text);
    pushTextCandidate(texts, choice?.message?.refusal);
    pushTextCandidate(texts, choice?.text);
  }

  pushTextCandidate(texts, response?.output_text);
  pushTextCandidate(texts, response?.output);

  return texts
    .map((item) => item.trim())
    .filter((item) => item.length > 0)
    .join("\n");
}

function pushTextCandidate(target, value) {
  if (value == null) {
    return;
  }

  if (typeof value === "string") {
    if (value.trim()) {
      target.push(value);
    }
    return;
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      pushTextCandidate(target, item);
    }
    return;
  }

  if (typeof value !== "object") {
    return;
  }

  if (typeof value.output_text === "string") {
    pushTextCandidate(target, value.output_text);
  }
  if (typeof value.content === "string") {
    pushTextCandidate(target, value.content);
  }
  if (typeof value.text === "string") {
    pushTextCandidate(target, value.text);
  }
  if (typeof value.refusal === "string") {
    pushTextCandidate(target, value.refusal);
  }
  if (typeof value.reasoning_content === "string") {
    pushTextCandidate(target, value.reasoning_content);
  }
  if (typeof value.value === "string") {
    pushTextCandidate(target, value.value);
  }

  if (value.text && typeof value.text === "object") {
    pushTextCandidate(target, value.text.value);
    pushTextCandidate(target, value.text.content);
    pushTextCandidate(target, value.text.text);
  }

  if (value.content && typeof value.content === "object") {
    pushTextCandidate(target, value.content.value);
    pushTextCandidate(target, value.content.text);
    pushTextCandidate(target, value.content.content);
  }

  if (value.message && typeof value.message === "object") {
    pushTextCandidate(target, value.message.content);
    pushTextCandidate(target, value.message.text);
    pushTextCandidate(target, value.message.reasoning_content);
    pushTextCandidate(target, value.message.refusal);
  }

  if (Array.isArray(value.content_parts)) {
    pushTextCandidate(target, value.content_parts);
  }

  if (Array.isArray(value.output)) {
    pushTextCandidate(target, value.output);
  }
}

function collectChatCompletionToolCalls(response) {
  return (response.choices ?? [])
    .flatMap((choice) => choice?.message?.tool_calls ?? [])
    .map((item) => ({
      id: item.id,
      name: item.function?.name,
      arguments: safeJsonParse(item.function?.arguments, item.function?.arguments ?? {}),
    }));
}

function describeEmptyChatCompletion(response, fallbackModel) {
  const choice = Array.isArray(response?.choices) ? response.choices[0] : undefined;
  const finishReason = choice?.finish_reason ?? "unknown";
  const model = response?.model ?? fallbackModel ?? "unknown";
  const responseId = response?.id ?? "unknown";
  const usage = summarizeUsage(response?.usage);
  const details = [
    `finish_reason=${finishReason}`,
    `model=${model}`,
    `response_id=${responseId}`,
    usage ? `usage=${usage}` : "",
  ].filter(Boolean).join(", ");
  return `OpenAI empty completion: ${details}`;
}

function summarizeUsage(usage) {
  if (!usage || typeof usage !== "object") {
    return "";
  }
  const promptTokens = typeof usage.prompt_tokens === "number" ? usage.prompt_tokens : null;
  const completionTokens = typeof usage.completion_tokens === "number" ? usage.completion_tokens : null;
  const totalTokens = typeof usage.total_tokens === "number" ? usage.total_tokens : null;
  const parts = [
    promptTokens == null ? "" : `prompt=${promptTokens}`,
    completionTokens == null ? "" : `completion=${completionTokens}`,
    totalTokens == null ? "" : `total=${totalTokens}`,
  ].filter(Boolean);
  return parts.join("/");
}

function summarizeOpenAiErrorBody(body) {
  const trimmed = String(body ?? "").trim();
  if (!trimmed) {
    return "";
  }

  try {
    const parsed = JSON.parse(trimmed);
    return (
      readJsonString(parsed, ["message"])
      || readJsonString(parsed, ["error"])
      || readJsonString(parsed, ["error", "message"])
      || readJsonString(parsed, ["top_reason"])
      || readJsonString(parsed, ["error", "code"])
      || readJsonString(parsed, ["code"])
      || readJsonArrayString(parsed, "failures", ["error_message"])
      || readJsonArrayString(parsed, "failures", ["top_reason"])
      || readJsonArrayString(parsed, "failures", ["error_code"])
      || ""
    );
  } catch {
    const lines = trimmed.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    for (let index = lines.length - 1; index >= 0; index -= 1) {
      const line = lines[index];
      if (/^(Error|panic|exception):/i.test(line)) {
        return line;
      }
    }
    return lines.at(-1) ?? trimmed;
  }
}

function readJsonString(value, path) {
  let current = value;
  for (const segment of path) {
    if (current == null || typeof current !== "object") {
      return "";
    }
    current = current[segment];
  }
  return typeof current === "string" ? current.trim() : "";
}

function readJsonArrayString(value, key, path) {
  const items = value?.[key];
  if (!Array.isArray(items)) {
    return "";
  }
  for (const item of items) {
    const found = readJsonString(item, path);
    if (found) {
      return found;
    }
  }
  return "";
}

function safeJsonParse(value, fallback) {
  if (typeof value !== "string") {
    return value ?? fallback;
  }
  try {
    return JSON.parse(value);
  } catch {
    return fallback;
  }
}

function trimTrailingSlash(value) {
  return value.replace(/\/+$/, "");
}

async function runStdio() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  if (chunks.length === 0) {
    return;
  }

  const request = JSON.parse(Buffer.concat(chunks).toString("utf8"));
  const result = request.method === "list_models"
    ? await listModels(request.params)
    : await generate(request.params);
  process.stdout.write(`${JSON.stringify({ ok: true, result })}\n`);
}

if (typeof process !== "undefined" && process.argv[1] && import.meta.url === `file://${process.argv[1].replace(/\\/g, "/")}`) {
  runStdio().catch((error) => {
    process.stdout.write(`${JSON.stringify({ ok: false, error: String(error?.message ?? error) })}\n`);
    process.exitCode = 1;
  });
}
