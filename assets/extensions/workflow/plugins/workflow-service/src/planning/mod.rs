use chrono::Utc;
use ennoia_kernel::{RunSpec, TaskKind, TaskSpec, TaskStatus};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;

const PLAN_SCHEMA_VERSION: &str = "3.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanSpec {
    #[serde(default = "default_plan_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub steps: Vec<PlanStep>,
    #[serde(default)]
    pub tool_plan: Vec<PlanTool>,
    #[serde(default)]
    pub verify_contract: JsonValue,
    #[serde(default)]
    pub delegation: JsonValue,
    #[serde(default)]
    pub watchdog: JsonValue,
    #[serde(default)]
    pub model_strategy: JsonValue,
    #[serde(default)]
    pub meta: PlanMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanStep {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub step_type: String,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub tool: String,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub input: JsonValue,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub expected_outputs: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub pass_if: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub next_pass: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub next_fail: String,
    #[serde(default)]
    pub assigned_agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanTool {
    #[serde(default)]
    pub tool: String,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub tools: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub allow_tools: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub allowed_tools: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub exec_heads: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    pub allow_exec_heads: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanMeta {
    #[serde(default)]
    pub plan_status: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub auto_generated: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanValidation {
    pub ready: bool,
    pub reason: String,
}

pub fn default_plan_schema_version() -> String {
    PLAN_SCHEMA_VERSION.to_string()
}

pub fn parse_plan_from_text(text: &str, objective: &str) -> Result<PlanSpec, String> {
    let blocks = extract_candidate_json_blocks(text);
    for block in blocks {
        if let Ok(plan) = serde_json::from_str::<PlanSpec>(&block) {
            return Ok(normalize_plan(plan, objective));
        }
    }
    Err("没有找到可解析的 plan.json".to_string())
}

pub fn normalize_plan(mut plan: PlanSpec, objective: &str) -> PlanSpec {
    let now = Utc::now().to_rfc3339();
    if plan.schema_version.trim().is_empty() || plan.schema_version.trim() == "1.0" {
        plan.schema_version = default_plan_schema_version();
    }
    if plan.objective.trim().is_empty() {
        plan.objective = objective.trim().to_string();
    }
    if plan.intent.trim().is_empty() {
        plan.intent = infer_intent(&plan);
    }
    if plan.meta.plan_status.trim().is_empty() {
        plan.meta.plan_status = "ready".to_string();
    }
    if plan.meta.created_at.trim().is_empty() {
        plan.meta.created_at = now.clone();
    }
    plan.meta.updated_at = now;

    for (index, step) in plan.steps.iter_mut().enumerate() {
        if step.id.trim().is_empty() {
            step.id = format!("S{}", index + 1);
        }
        step.step_type = normalize_step_type(&step.step_type);
        step.goal = step.goal.trim().to_string();
        step.tool = step.tool.trim().to_string();
        normalize_string_list(&mut step.allowed_tools);
        if step.allowed_tools.is_empty() && !step.tool.is_empty() {
            step.allowed_tools.push(step.tool.clone());
        }
        normalize_string_list(&mut step.expected_outputs);
        normalize_string_list(&mut step.pass_if);
        step.next_pass = step.next_pass.trim().to_string();
        step.next_fail = step.next_fail.trim().to_string();
        step.assigned_agent_id = step.assigned_agent_id.trim().to_string();
    }

    for item in &mut plan.tool_plan {
        item.tool = item.tool.trim().to_string();
        normalize_string_list(&mut item.tools);
        normalize_string_list(&mut item.allow_tools);
        normalize_string_list(&mut item.allowed_tools);
        normalize_string_list(&mut item.exec_heads);
        normalize_string_list(&mut item.allow_exec_heads);
    }

    if plan.verify_contract.is_null() {
        plan.verify_contract = default_verify_contract(&plan.intent);
    }
    plan
}

pub fn validate_plan(plan: &PlanSpec) -> PlanValidation {
    if plan.schema_version.trim() != PLAN_SCHEMA_VERSION {
        return invalid("schema_version 必须为 3.0");
    }
    if plan.objective.trim().is_empty() {
        return invalid("objective 不能为空");
    }
    if plan.steps.is_empty() {
        return invalid("steps 为空，无法进入执行阶段");
    }
    if plan.meta.plan_status.trim().to_ascii_lowercase() != "ready" {
        return invalid("plan_status 必须为 ready");
    }
    if plan.verify_contract.is_null() || !plan.verify_contract.is_object() {
        return invalid("verify_contract 缺失，无法进入执行阶段");
    }

    let concrete_tools = collect_concrete_tools(plan);
    if concrete_tools.is_empty() {
        return invalid("tool_plan 未声明具体工具");
    }

    if plan.steps.iter().any(|step| step.id.trim().is_empty()) {
        return invalid("steps 存在缺失 id 的步骤");
    }

    let is_mutation = matches!(
        plan.intent.trim().to_ascii_lowercase().as_str(),
        "code_change" | "cloud_change" | "mixed"
    );
    if is_mutation {
        let baseline_index = plan
            .steps
            .iter()
            .position(|step| step.step_type == "baseline_verify");
        let execute_index = plan.steps.iter().position(|step| {
            matches!(
                step.step_type.as_str(),
                "execute" | "change" | "write" | "cloud_change"
            )
        });
        if baseline_index.is_none() {
            return invalid("修改类任务缺少 baseline_verify 步骤");
        }
        if let (Some(baseline_index), Some(execute_index)) = (baseline_index, execute_index) {
            if baseline_index > execute_index {
                return invalid("baseline_verify 必须在改动步骤前");
            }
        }
    }

    let search_like = plan.intent.trim().to_ascii_lowercase().contains("search")
        || plan
            .steps
            .iter()
            .any(|step| step.step_type.starts_with("search"));
    if search_like
        && !plan.steps.iter().any(|step| {
            matches!(
                step.step_type.as_str(),
                "search_online" | "search_local" | "search"
            )
        })
    {
        return invalid("搜索类任务缺少 search 步骤");
    }

    PlanValidation {
        ready: true,
        reason: String::new(),
    }
}

pub fn derive_tasks_from_plan(
    run: &RunSpec,
    plan: &PlanSpec,
    default_agent_id: &str,
    task_kind: TaskKind,
) -> Vec<TaskSpec> {
    plan.steps
        .iter()
        .enumerate()
        .map(|(index, step)| TaskSpec {
            id: format!("task-{}-{}", run.id, index + 1),
            run_id: run.id.clone(),
            conversation_id: run.conversation_id.clone(),
            lane_id: run.lane_id.clone(),
            task_kind,
            title: if step.goal.trim().is_empty() {
                humanize_step_type(&step.step_type)
            } else {
                step.goal.trim().to_string()
            },
            assigned_agent_id: if step.assigned_agent_id.trim().is_empty() {
                default_agent_id.to_string()
            } else {
                step.assigned_agent_id.trim().to_string()
            },
            status: TaskStatus::Pending,
            created_at: run.created_at.clone(),
            updated_at: run.updated_at.clone(),
        })
        .collect()
}

pub fn summarize_plan_steps(plan: &PlanSpec) -> Vec<String> {
    plan.steps
        .iter()
        .filter_map(|step| {
            let goal = step.goal.trim();
            if goal.is_empty() {
                None
            } else {
                Some(goal.to_string())
            }
        })
        .collect()
}

pub fn build_planning_prompt(
    goal: &str,
    require_confirmation: bool,
    acceptance_first: bool,
) -> String {
    let confirm_line = if require_confirmation {
        "本轮只输出可执行计划，不要直接开始执行，等待用户确认。"
    } else {
        "计划就绪后可以继续进入执行。"
    };
    let acceptance_line = if acceptance_first {
        "验收先行：先定义完成标准，并写入 verify_contract 和每个步骤的 pass_if；执行结束后必须按完成标准检查结果。"
    } else {
        "根据任务风险设置 verify_contract 和 pass_if，保证计划可检查。"
    };
    format!(
        "你现在负责为当前任务生成一份可执行计划。\n\
任务目标：{}\n\
要求：\n\
1. 先用简短自然语言概述执行思路。\n\
2. 然后输出一个 ```json``` 代码块，内容必须是可执行 plan。\n\
3. plan 必须包含：schema_version、objective、intent、steps、tool_plan、verify_contract、meta。\n\
4. schema_version 必须为 3.0。\n\
5. steps 每项至少包含：id、type、goal、allowed_tools、expected_outputs、pass_if、next_pass、next_fail；终止步骤的 next_pass/next_fail 使用空字符串，不要使用 null。\n\
6. 如果任务涉及修改代码或文件，必须包含 baseline_verify 步骤，并放在写操作之前。\n\
7. tool_plan 必须写本次真正会使用的具体工具，不要留空，不要用占位符。\n\
8. meta.plan_status 必须为 ready。\n\
9. 除了自然语言说明和一个 JSON 代码块，不要输出其他杂项。\n\
10. {}\n\
{}\n",
        goal.trim(),
        acceptance_line,
        confirm_line
    )
}

fn extract_candidate_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index + 2 < bytes.len() {
        if &bytes[index..index + 3] == b"```" {
            let mut cursor = index + 3;
            while cursor < bytes.len() && (bytes[cursor] == b' ' || bytes[cursor] == b'\t') {
                cursor += 1;
            }
            if cursor + 4 <= bytes.len() && text[cursor..].starts_with("json") {
                cursor += 4.min(bytes.len().saturating_sub(cursor));
            }
            while cursor < bytes.len() && (bytes[cursor] == b'\r' || bytes[cursor] == b'\n') {
                cursor += 1;
            }
            if let Some(close_offset) = text[cursor..].find("```") {
                let block = text[cursor..cursor + close_offset].trim();
                if !block.is_empty() {
                    blocks.push(block.to_string());
                }
                index = cursor + close_offset + 3;
                continue;
            }
        }
        index += 1;
    }

    let trimmed = text.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        blocks.push(trimmed.to_string());
    }
    blocks
}

fn deserialize_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    Ok(json_value_tokens(&value))
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    Ok(match value {
        JsonValue::String(text) => text,
        JsonValue::Null => String::new(),
        other => other.to_string(),
    })
}

fn json_value_tokens(value: &JsonValue) -> Vec<String> {
    let mut items = Vec::new();
    match value {
        JsonValue::String(text) => {
            items.extend(
                text.split([',', '\n'])
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
        JsonValue::Array(values) => {
            for value in values {
                items.extend(json_value_tokens(value));
            }
        }
        _ => {}
    }
    let mut deduped = Vec::new();
    for item in items {
        if !deduped.iter().any(|existing| existing == &item) {
            deduped.push(item);
        }
    }
    deduped
}

fn normalize_string_list(items: &mut Vec<String>) {
    let mut next = Vec::new();
    for item in items.drain(..) {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !next.iter().any(|existing| existing == trimmed) {
            next.push(trimmed.to_string());
        }
    }
    *items = next;
}

fn normalize_step_type(value: &str) -> String {
    let raw = value.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match raw.as_str() {
        "" => "execute".to_string(),
        "search" | "search_remote" => "search_online".to_string(),
        "local_search" => "search_local".to_string(),
        "baseline" | "baseline_check" | "baseline_verification" => "baseline_verify".to_string(),
        "validation" | "verify" => "validate".to_string(),
        other => other.to_string(),
    }
}

fn collect_concrete_tools(plan: &PlanSpec) -> Vec<String> {
    let mut tools = Vec::new();
    for item in &plan.tool_plan {
        push_tool_token(&mut tools, &item.tool);
        for token in &item.tools {
            push_tool_token(&mut tools, token);
        }
        for token in &item.allow_tools {
            push_tool_token(&mut tools, token);
        }
        for token in &item.allowed_tools {
            push_tool_token(&mut tools, token);
        }
        for token in &item.exec_heads {
            push_tool_token(&mut tools, token);
        }
        for token in &item.allow_exec_heads {
            push_tool_token(&mut tools, token);
        }
    }
    for step in &plan.steps {
        push_tool_token(&mut tools, &step.tool);
        for token in &step.allowed_tools {
            push_tool_token(&mut tools, token);
        }
    }
    tools
}

fn push_tool_token(target: &mut Vec<String>, raw: &str) {
    let token = raw.trim();
    if token.is_empty() || token == "*" {
        return;
    }
    if !target.iter().any(|existing| existing == token) {
        target.push(token.to_string());
    }
}

fn infer_intent(plan: &PlanSpec) -> String {
    if plan
        .steps
        .iter()
        .any(|step| step.step_type.starts_with("search"))
    {
        return "search_online".to_string();
    }
    if plan.steps.iter().any(|step| {
        matches!(
            step.step_type.as_str(),
            "baseline_verify" | "change" | "write"
        )
    }) {
        return "code_change".to_string();
    }
    "mixed".to_string()
}

fn default_verify_contract(intent: &str) -> JsonValue {
    serde_json::json!({
        "intent": intent,
        "baseline_required_for_mutation": matches!(intent, "code_change" | "cloud_change" | "mixed"),
        "evidence_required": intent.contains("search"),
        "effectiveness_check": {
            "require_effective": true,
            "require_impactful": true
        }
    })
}

fn invalid(reason: impl Into<String>) -> PlanValidation {
    PlanValidation {
        ready: false,
        reason: reason.into(),
    }
}

fn humanize_step_type(step_type: &str) -> String {
    match step_type {
        "plan_confirm" => "确认计划".to_string(),
        "baseline_verify" => "核实现状".to_string(),
        "search_local" => "本地检索".to_string(),
        "search_online" => "线上检索".to_string(),
        "validate" => "结果验证".to_string(),
        "execute" => "执行实施".to_string(),
        other => other.replace('_', " "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plan_from_json_fence() {
        let plan = parse_plan_from_text(
            "方案如下\n```json\n{\"schema_version\":\"3.0\",\"objective\":\"测试\",\"intent\":\"mixed\",\"steps\":[{\"id\":\"S1\",\"type\":\"baseline_verify\",\"goal\":\"看现状\",\"allowed_tools\":[\"command_exec\"],\"expected_outputs\":[\"baseline\"],\"pass_if\":[\"ok\"],\"next_pass\":\"S2\",\"next_fail\":\"REPLAN\"}],\"tool_plan\":[{\"tool\":\"command_exec\"}],\"verify_contract\":{\"ok\":true},\"meta\":{\"plan_status\":\"ready\"}}\n```",
            "测试",
        )
        .expect("plan");
        assert_eq!(plan.objective, "测试");
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn normalizes_legacy_generated_plan_version() {
        let plan = parse_plan_from_text(
            "方案如下\n```json\n{\"schema_version\":\"1.0\",\"objective\":\"写博客\",\"intent\":\"write_blog\",\"steps\":[{\"id\":\"S1\",\"type\":\"draft\",\"goal\":\"写草稿\",\"allowed_tools\":[],\"expected_outputs\":[\"草稿\"],\"pass_if\":\"内容完整\",\"next_pass\":null,\"next_fail\":\"S1\"}],\"tool_plan\":[{\"tool\":\"command_exec\"}],\"verify_contract\":{\"ok\":true},\"meta\":{\"plan_status\":\"ready\"}}\n```",
            "写博客",
        )
        .expect("legacy plan should parse");

        assert_eq!(plan.schema_version, default_plan_schema_version());
        assert_eq!(plan.steps[0].next_pass, "");
        assert!(validate_plan(&plan).ready);
    }

    #[test]
    fn planning_prompt_names_current_schema_version() {
        let prompt = build_planning_prompt("写博客", true, false);

        assert!(prompt.contains("schema_version 必须为 3.0"));
    }

    #[test]
    fn acceptance_first_planning_prompt_requires_completion_criteria() {
        let prompt = build_planning_prompt("写博客", true, true);

        assert!(prompt.contains("先定义完成标准"));
        assert!(prompt.contains("执行结束后必须按完成标准检查"));
    }

    #[test]
    fn validates_mutation_plan_requires_baseline() {
        let plan = normalize_plan(
            PlanSpec {
                schema_version: "3.0".to_string(),
                objective: "改代码".to_string(),
                intent: "code_change".to_string(),
                steps: vec![PlanStep {
                    id: "S1".to_string(),
                    step_type: "execute".to_string(),
                    goal: "直接改".to_string(),
                    tool: String::new(),
                    allowed_tools: vec!["command_exec".to_string()],
                    input: JsonValue::Null,
                    expected_outputs: vec![],
                    pass_if: vec![],
                    next_pass: "DONE".to_string(),
                    next_fail: "REPLAN".to_string(),
                    assigned_agent_id: String::new(),
                }],
                tool_plan: vec![PlanTool {
                    tool: "command_exec".to_string(),
                    tools: vec![],
                    allow_tools: vec![],
                    allowed_tools: vec![],
                    exec_heads: vec![],
                    allow_exec_heads: vec![],
                }],
                verify_contract: serde_json::json!({ "ok": true }),
                delegation: JsonValue::Null,
                watchdog: JsonValue::Null,
                model_strategy: JsonValue::Null,
                meta: PlanMeta {
                    plan_status: "ready".to_string(),
                    source: String::new(),
                    auto_generated: false,
                    created_at: String::new(),
                    updated_at: String::new(),
                },
            },
            "改代码",
        );
        let verdict = validate_plan(&plan);
        assert!(!verdict.ready);
        assert!(verdict.reason.contains("baseline_verify"));
    }
}
