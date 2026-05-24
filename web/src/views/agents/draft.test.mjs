import { describe, expect, test } from "bun:test";

import {
  createDefaultAgentDraft,
  shouldInitializeAgentDraft,
} from "./draft.ts";

const openaiEndpoint = {
  id: "openai",
  display_name: "OpenAI",
  kind: "openai",
  description: "",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  api_key_env: "",
  request_timeout_ms: 0,
  default_model: "gpt-5.4",
  available_models: [{ id: "gpt-5.4" }],
  model_discovery: { manual_allowed: true },
  enabled: true,
};

const openaiContribution = {
  extension_id: "openai",
  provider: {
    id: "openai",
    kind: "openai",
    interfaces: ["generate", "models"],
    model_discovery: true,
    manual_model: true,
    generation_options: [
      {
        id: "reasoning_effort",
        label: { key: "ext.openai.option.reasoning_effort", fallback: "Reasoning effort" },
        value_type: "select",
        required: false,
        default_value: "medium",
        allowed_values: ["low", "medium", "high", "xhigh"],
      },
    ],
  },
};

describe("agent draft initialization", () => {
  test("creates a new Agent draft from the first model endpoint", () => {
    const draft = createDefaultAgentDraft([openaiEndpoint], [openaiContribution]);

    expect(draft.model_endpoint_id).toBe("openai");
    expect(draft.model_id).toBe("gpt-5.4");
    expect(draft.generation_options).toEqual({ reasoning_effort: "medium" });
  });

  test("does not reinitialize a draft that already belongs to the current new Agent tab", () => {
    expect(shouldInitializeAgentDraft({
      agentId: "new-1",
      initializedAgentId: "new-1",
    })).toBe(false);
  });

  test("reinitializes when the user opens another new Agent tab", () => {
    expect(shouldInitializeAgentDraft({
      agentId: "new-2",
      initializedAgentId: "new-1",
    })).toBe(true);
  });
});
