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
    input: normalizeInput(request.messages ?? request.input ?? request.prompt ?? ""),
  };

  const instructions = request.instructions ?? request.system_prompt;
  if (instructions) {
    payload.instructions = instructions;
  }

  const tools = normalizeTools(request.tools ?? []);
  if (tools.length > 0) {
    payload.tools = tools;
    payload.tool_choice = request.tool_choice ?? "auto";
  }

  const reasoningEffort = request.generation_options?.reasoning_effort ?? request.reasoning_effort;
  if (reasoningEffort) {
    payload.reasoning = { effort: reasoningEffort };
  }

  if (request.metadata && typeof request.metadata === "object") {
    payload.metadata = request.metadata;
  }

  const response = await openaiFetch(config, "/responses", {
    method: "POST",
    body: JSON.stringify(payload),
  });
  const data = await response.json();

  return {
    id: data.id,
    model: data.model ?? model,
    text: collectOutputText(data),
    tool_calls: collectToolCalls(data),
    raw: data,
  };
}

function normalizeProviderConfig(config) {
  const baseUrl = trimTrailingSlash(config.base_url || DEFAULT_BASE_URL);
  const apiKeyEnv = config.api_key_env || "OPENAI_API_KEY";
  const apiKey = config.api_key || process.env[apiKeyEnv];
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
      "authorization": `Bearer ${config.api_key}`,
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

function normalizeInput(input) {
  if (typeof input === "string") {
    return input;
  }
  if (!Array.isArray(input)) {
    return String(input ?? "");
  }

  return input.map((message) => ({
    role: normalizeRole(message.role ?? message.sender),
    content: normalizeContent(message.content ?? message.body ?? message.text ?? ""),
  }));
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

function normalizeContent(content) {
  if (typeof content === "string") {
    return content;
  }
  if (Array.isArray(content)) {
    return content;
  }
  return String(content ?? "");
}

function normalizeTools(tools) {
  return tools.map((tool) => {
    if (tool.type) {
      return tool;
    }
    return {
      type: "function",
      name: tool.name,
      description: tool.description ?? "",
      parameters: tool.parameters ?? tool.input_schema ?? { type: "object", properties: {} },
      strict: tool.strict ?? false,
    };
  });
}

function collectOutputText(response) {
  if (typeof response.output_text === "string") {
    return response.output_text;
  }

  return (response.output ?? [])
    .flatMap((item) => item.content ?? [])
    .filter((part) => part.type === "output_text" || part.type === "text")
    .map((part) => part.text ?? "")
    .join("");
}

function collectToolCalls(response) {
  return (response.output ?? [])
    .filter((item) => item.type === "function_call")
    .map((item) => ({
      id: item.call_id ?? item.id,
      name: item.name,
      arguments: safeJsonParse(item.arguments, item.arguments ?? {}),
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
