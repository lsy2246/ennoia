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
