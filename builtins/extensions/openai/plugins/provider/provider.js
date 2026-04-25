const DEFAULT_BASE_URL = "https://api.openai.com/v1";
const DEFAULT_MODEL = "gpt-5.4";

export const provider = {
  id: "openai",
  kind: "openai",
  interfaces: ["generate", "tools", "models"],
  recommendedModels: {
    openai: DEFAULT_MODEL,
  },
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
        .map((item) => item?.id)
        .filter((item) => typeof item === "string")
        .sort()
    : [];

  return {
    provider_id: config.id,
    models,
    recommended_model: config.default_model || DEFAULT_MODEL,
  };
}

export async function generate(request = {}) {
  const config = normalizeProviderConfig(request.provider ?? {});
  const model = request.model ?? config.default_model ?? DEFAULT_MODEL;
  const payload = {
    model,
    messages: toChatCompletionMessages(
      request.messages ?? request.input ?? request.prompt ?? "",
      request.instructions ?? request.system_prompt,
    ),
  };

  const tools = normalizeChatCompletionTools(request.tools ?? []);
  if (tools.length > 0) {
    payload.tools = tools;
    payload.tool_choice = request.tool_choice ?? "auto";
  }

  if (request.metadata && typeof request.metadata === "object") {
    payload.metadata = request.metadata;
  }

  const response = await openaiFetch(config, "/chat/completions", {
    method: "POST",
    body: JSON.stringify(payload),
  });
  const data = await response.json();
  const text = collectChatCompletionText(data);
  const toolCalls = collectChatCompletionToolCalls(data);

  if (!text && toolCalls.length === 0) {
    throw new Error(`OpenAI response missing assistant text: ${JSON.stringify(data)}`);
  }

  return {
    id: data.id,
    model: data.model ?? model,
    text,
    tool_calls: toolCalls,
    raw: data,
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
    throw new Error(`OpenAI request failed: ${response.status} ${body}`);
  }

  return response;
}

function toChatCompletionMessages(input, instructions) {
  const messages = [];
  if (instructions) {
    messages.push({ role: "system", content: String(instructions) });
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

function collectChatCompletionText(response) {
  return (response.choices ?? [])
    .map((choice) => normalizeAssistantContent(choice?.message?.content))
    .filter((item) => typeof item === "string" && item.trim().length > 0)
    .join("\n");
}

function normalizeAssistantContent(content) {
  if (typeof content === "string") {
    return content;
  }
  if (!Array.isArray(content)) {
    return "";
  }
  return content
    .map((part) => {
      if (typeof part === "string") {
        return part;
      }
      if (typeof part?.text === "string") {
        return part.text;
      }
      if (typeof part?.content === "string") {
        return part.content;
      }
      return "";
    })
    .filter((item) => item.trim().length > 0)
    .join("\n");
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
