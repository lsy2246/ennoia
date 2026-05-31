#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import test from "node:test";

import { provider } from "./provider.js";

test("OpenAI generation options expose the highest reasoning effort", () => {
  const reasoningEffort = provider.generationOptions.find(
    (option) => option.id === "reasoning_effort",
  );

  assert(reasoningEffort, "reasoning_effort should be declared");
  assert.deepEqual(reasoningEffort.values, ["low", "medium", "high", "xhigh"]);
});

test("OpenAI extension manifest exposes the highest reasoning effort to the UI", () => {
  const manifestPath = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../../extension.toml",
  );
  const manifest = readFileSync(manifestPath, "utf8");

  assert.match(
    manifest,
    /allowed_values\s*=\s*\["low",\s*"medium",\s*"high",\s*"xhigh"\]/,
  );
});

test("OpenAI endpoint preset does not configure an API key environment variable", () => {
  const presetPath = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../../model-endpoint-presets/openai.toml",
  );
  const preset = readFileSync(presetPath, "utf8");

  assert.equal(preset.includes("api_key_env"), false);
});

test("OpenAI config does not read OPENAI_API_KEY unless api_key_env is explicit", async () => {
  const previous = process.env.OPENAI_API_KEY;
  process.env.OPENAI_API_KEY = "env-only-key";

  try {
    await assert.rejects(
      () => provider.listModels({
        base_url: "http://127.0.0.1:9",
        api_key: "",
        api_key_env: "",
      }),
      /OpenAI API key is missing/,
    );
  } finally {
    if (previous === undefined) {
      delete process.env.OPENAI_API_KEY;
    } else {
      process.env.OPENAI_API_KEY = previous;
    }
  }
});

test("OpenAI provider splits think-tagged output before returning generation text", async () => {
  const previousFetch = globalThis.fetch;
  globalThis.fetch = async () => ({
    ok: true,
    json: async () => ({
      id: "chatcmpl-test",
      model: "test-model",
      choices: [{
        message: {
          content: "<think>Exploring screenshot options\n\nI need to test this out!</think>\nFinal answer.",
        },
      }],
    }),
  });

  try {
    const result = await provider.generate({
      model_endpoint: {
        base_url: "http://127.0.0.1:4321/v1",
        api_key: "test-key",
      },
      model: "test-model",
      tools: [{
        name: "noop",
        description: "No-op tool used to force non-streaming generation in this test.",
        parameters: { type: "object", properties: {} },
      }],
      messages: "hello",
    });

    assert.equal(result.text, "Final answer.");
    assert.equal(result.reasoning, "Exploring screenshot options\n\nI need to test this out!");
  } finally {
    globalThis.fetch = previousFetch;
  }
});

test("OpenAI provider falls back to non-streaming when stream read resets", async () => {
  const previousFetch = globalThis.fetch;
  let callCount = 0;
  globalThis.fetch = async () => {
    callCount += 1;
    if (callCount === 1) {
      return {
        ok: true,
        body: {
          getReader: () => ({
            read: async () => {
              const error = new Error("read ECONNRESET");
              error.code = "ECONNRESET";
              error.syscall = "read";
              throw error;
            },
          }),
        },
      };
    }
    return {
      ok: true,
      json: async () => ({
        id: "chatcmpl-fallback",
        model: "test-model",
        choices: [{
          message: {
            content: "Recovered without streaming.",
          },
        }],
      }),
    };
  };

  try {
    const result = await provider.generate({
      model_endpoint: {
        base_url: "http://127.0.0.1:4321/v1",
        api_key: "test-key",
      },
      model: "test-model",
      messages: "hello",
    });

    assert.equal(callCount, 2);
    assert.equal(result.text, "Recovered without streaming.");
  } finally {
    globalThis.fetch = previousFetch;
  }
});

test("OpenAI provider uses streaming even when tools are available", async () => {
  const previousFetch = globalThis.fetch;
  let callCount = 0;
  globalThis.fetch = async (_url, init = {}) => {
    callCount += 1;
    const payload = JSON.parse(init.body);
    assert.equal(payload.stream, true);
    assert.equal(Array.isArray(payload.tools), true);
    return {
      ok: true,
      body: streamFromSseEvents([
        {
          id: "chatcmpl-tool-stream",
          model: "test-model",
          choices: [{
            delta: {
              content: "Recovered with streaming tools.",
            },
          }],
        },
        {
          id: "chatcmpl-tool-stream",
          model: "test-model",
          choices: [{
            finish_reason: "stop",
            delta: {},
          }],
        },
      ]),
    };
  };

  try {
    const result = await provider.generate({
      model_endpoint: {
        base_url: "http://127.0.0.1:4321/v1",
        api_key: "test-key",
      },
      model: "test-model",
      tools: [{
        name: "noop",
        description: "No-op tool.",
        parameters: { type: "object", properties: {} },
      }],
      messages: "hello",
    });

    assert.equal(callCount, 1);
    assert.equal(result.text, "Recovered with streaming tools.");
    assert.deepEqual(result.tool_calls, []);
  } finally {
    globalThis.fetch = previousFetch;
  }
});

test("OpenAI provider parses streamed tool calls without non-streaming fallback", async () => {
  const previousFetch = globalThis.fetch;
  let callCount = 0;
  globalThis.fetch = async (_url, init = {}) => {
    callCount += 1;
    const payload = JSON.parse(init.body);
    assert.equal(payload.stream, true);
    assert.equal(Array.isArray(payload.tools), true);
    return {
      ok: true,
      body: streamFromSseEvents([
        {
          id: "chatcmpl-tool-call-stream",
          model: "test-model",
          choices: [{
            delta: {
              tool_calls: [{
                index: 0,
                id: "call-1",
                type: "function",
                function: {
                  name: "noop",
                  arguments: "{\"ok\"",
                },
              }],
            },
          }],
        },
        {
          id: "chatcmpl-tool-call-stream",
          model: "test-model",
          choices: [{
            delta: {
              tool_calls: [{
                index: 0,
                function: {
                  arguments: ":true}",
                },
              }],
            },
          }],
        },
        {
          id: "chatcmpl-tool-call-stream",
          model: "test-model",
          choices: [{
            finish_reason: "tool_calls",
            delta: {},
          }],
        },
      ]),
    };
  };

  try {
    const result = await provider.generate({
      model_endpoint: {
        base_url: "http://127.0.0.1:4321/v1",
        api_key: "test-key",
      },
      model: "test-model",
      tools: [{
        name: "noop",
        description: "No-op tool.",
        parameters: { type: "object", properties: {} },
      }],
      messages: "hello",
    });

    assert.equal(callCount, 1);
    assert.equal(result.text, "");
    assert.deepEqual(result.tool_calls, [{
      id: "call-1",
      name: "noop",
      arguments: { ok: true },
    }]);
  } finally {
    globalThis.fetch = previousFetch;
  }
});

test("OpenAI provider parses response output text stream events", async () => {
  const previousFetch = globalThis.fetch;
  let callCount = 0;
  globalThis.fetch = async (_url, init = {}) => {
    callCount += 1;
    const payload = JSON.parse(init.body);
    assert.equal(payload.stream, true);
    return {
      ok: true,
      body: streamFromSseEvents([
        {
          type: "response.output_text.delta",
          delta: "Hello ",
        },
        {
          type: "response.output_text.delta",
          delta: "from responses stream.",
        },
      ]),
    };
  };

  try {
    const result = await provider.generate({
      model_endpoint: {
        base_url: "http://127.0.0.1:4321/v1",
        api_key: "test-key",
      },
      model: "test-model",
      messages: "hello",
    });

    assert.equal(callCount, 1);
    assert.equal(result.text, "Hello from responses stream.");
  } finally {
    globalThis.fetch = previousFetch;
  }
});

test("OpenAI provider does not retry non-streaming after a parsed stream with no content", async () => {
  const previousFetch = globalThis.fetch;
  let callCount = 0;
  globalThis.fetch = async (_url, init = {}) => {
    callCount += 1;
    const payload = JSON.parse(init.body);
    assert.equal(payload.stream, true);
    return {
      ok: true,
      body: streamFromSseEvents([
        {
          id: "chatcmpl-empty-stream",
          model: "test-model",
          choices: [{
            finish_reason: "stop",
            delta: {},
          }],
        },
      ]),
    };
  };

  try {
    await assert.rejects(
      () => provider.generate({
        model_endpoint: {
          base_url: "http://127.0.0.1:4321/v1",
          api_key: "test-key",
        },
        model: "test-model",
        messages: "hello",
      }),
      /OpenAI empty streamed completion/,
    );

    assert.equal(callCount, 1);
  } finally {
    globalThis.fetch = previousFetch;
  }
});

function streamFromSseEvents(events) {
  const encoder = new TextEncoder();
  const chunks = [
    ...events.map((event) => `data: ${JSON.stringify(event)}\n\n`),
    "data: [DONE]\n\n",
  ].map((chunk) => encoder.encode(chunk));
  let index = 0;
  return {
    getReader: () => ({
      read: async () => {
        if (index >= chunks.length) {
          return { done: true };
        }
        return { done: false, value: chunks[index++] };
      },
    }),
  };
}
