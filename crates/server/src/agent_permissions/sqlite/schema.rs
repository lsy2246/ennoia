#[derive(Debug, Clone, Copy)]
pub enum SqlType {
    Integer,
    Text,
}

impl SqlType {
    fn render(self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Text => "TEXT",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnDef {
    pub name: &'static str,
    pub sql_type: SqlType,
    pub constraints: &'static [&'static str],
}

impl ColumnDef {
    pub fn render_definition(self) -> String {
        let mut definition = format!("{} {}", self.name, self.sql_type.render());
        if !self.constraints.is_empty() {
            definition.push(' ');
            definition.push_str(&self.constraints.join(" "));
        }
        definition
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IndexColumn {
    pub name: &'static str,
    pub descending: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct IndexDef {
    pub name: &'static str,
    pub columns: &'static [IndexColumn],
}

pub trait TableSchema {
    const NAME: &'static str;
    const COLUMNS: &'static [ColumnDef];
    const INDEXES: &'static [IndexDef];
    const INSERT_COLUMNS: &'static [&'static str];
    const SELECT_COLUMNS: &'static [&'static str];

    fn create_table_statement() -> String {
        let columns = Self::COLUMNS
            .iter()
            .map(|column| column.render_definition())
            .collect::<Vec<_>>()
            .join(",\n  ");
        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n  {}\n);",
            Self::NAME,
            columns
        )
    }

    fn create_index_statements() -> Vec<String> {
        Self::INDEXES
            .iter()
            .map(|index| {
                let columns = index
                    .columns
                    .iter()
                    .map(|column| {
                        if column.descending {
                            format!("{} DESC", column.name)
                        } else {
                            column.name.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "CREATE INDEX IF NOT EXISTS {} ON {}({});",
                    index.name,
                    Self::NAME,
                    columns
                )
            })
            .collect()
    }

    fn insert_statement() -> String {
        let placeholders = (1..=Self::INSERT_COLUMNS.len())
            .map(|index| format!("?{}", index))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            Self::NAME,
            Self::INSERT_COLUMNS.join(", "),
            placeholders,
        )
    }
}

pub struct PermissionEventsSchema;

impl PermissionEventsSchema {
    pub const SEQ: &'static str = "seq";
    pub const EVENT_ID: &'static str = "event_id";
    pub const AGENT_ID: &'static str = "agent_id";
    pub const ACTION: &'static str = "action";
    pub const DECISION: &'static str = "decision";
    pub const TARGET_JSON: &'static str = "target_json";
    pub const SCOPE_JSON: &'static str = "scope_json";
    pub const EXTENSION_ID: &'static str = "extension_id";
    pub const MATCHED_RULE_ID: &'static str = "matched_rule_id";
    pub const APPROVAL_ID: &'static str = "approval_id";
    pub const TRACE_ID: &'static str = "trace_id";
    pub const CREATED_AT: &'static str = "created_at";
}

impl TableSchema for PermissionEventsSchema {
    const NAME: &'static str = "permission_events";
    const COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::SEQ,
            sql_type: SqlType::Integer,
            constraints: &["PRIMARY KEY", "AUTOINCREMENT"],
        },
        ColumnDef {
            name: Self::EVENT_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL", "UNIQUE"],
        },
        ColumnDef {
            name: Self::AGENT_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::ACTION,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::DECISION,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::TARGET_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SCOPE_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::EXTENSION_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::MATCHED_RULE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::APPROVAL_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::TRACE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::CREATED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
    ];
    const INDEXES: &'static [IndexDef] = &[
        IndexDef {
            name: "idx_permission_events_agent_time",
            columns: &[
                IndexColumn {
                    name: Self::AGENT_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
        IndexDef {
            name: "idx_permission_events_decision_time",
            columns: &[
                IndexColumn {
                    name: Self::DECISION,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        Self::EVENT_ID,
        Self::AGENT_ID,
        Self::ACTION,
        Self::DECISION,
        Self::TARGET_JSON,
        Self::SCOPE_JSON,
        Self::EXTENSION_ID,
        Self::MATCHED_RULE_ID,
        Self::APPROVAL_ID,
        Self::TRACE_ID,
        Self::CREATED_AT,
    ];
    const SELECT_COLUMNS: &'static [&'static str] = &[
        Self::EVENT_ID,
        Self::AGENT_ID,
        Self::ACTION,
        Self::DECISION,
        Self::TARGET_JSON,
        Self::SCOPE_JSON,
        Self::EXTENSION_ID,
        Self::MATCHED_RULE_ID,
        Self::APPROVAL_ID,
        Self::TRACE_ID,
        Self::CREATED_AT,
    ];
}

pub struct PermissionApprovalsSchema;

impl PermissionApprovalsSchema {
    pub const SEQ: &'static str = "seq";
    pub const APPROVAL_ID: &'static str = "approval_id";
    pub const STATUS: &'static str = "status";
    pub const AGENT_ID: &'static str = "agent_id";
    pub const ACTION: &'static str = "action";
    pub const CONVERSATION_ID: &'static str = "conversation_id";
    pub const RUN_ID: &'static str = "run_id";
    pub const MESSAGE_ID: &'static str = "message_id";
    pub const TARGET_JSON: &'static str = "target_json";
    pub const SCOPE_JSON: &'static str = "scope_json";
    pub const TRIGGER_JSON: &'static str = "trigger_json";
    pub const MATCHED_RULE_ID: &'static str = "matched_rule_id";
    pub const REASON: &'static str = "reason";
    pub const CREATED_AT: &'static str = "created_at";
    pub const EXPIRES_AT: &'static str = "expires_at";
    pub const RESOLVED_AT: &'static str = "resolved_at";
    pub const RESOLUTION: &'static str = "resolution";

    pub const LEGACY_COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::CONVERSATION_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::RUN_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::MESSAGE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::EXPIRES_AT,
            sql_type: SqlType::Text,
            constraints: &[],
        },
    ];
}

impl TableSchema for PermissionApprovalsSchema {
    const NAME: &'static str = "permission_approvals";
    const COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::SEQ,
            sql_type: SqlType::Integer,
            constraints: &["PRIMARY KEY", "AUTOINCREMENT"],
        },
        ColumnDef {
            name: Self::APPROVAL_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL", "UNIQUE"],
        },
        ColumnDef {
            name: Self::STATUS,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::AGENT_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::ACTION,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::CONVERSATION_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::RUN_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::MESSAGE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::TARGET_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SCOPE_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::TRIGGER_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::MATCHED_RULE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::REASON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::CREATED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::EXPIRES_AT,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::RESOLVED_AT,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::RESOLUTION,
            sql_type: SqlType::Text,
            constraints: &[],
        },
    ];
    const INDEXES: &'static [IndexDef] = &[
        IndexDef {
            name: "idx_permission_approvals_status_time",
            columns: &[
                IndexColumn {
                    name: Self::STATUS,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
        IndexDef {
            name: "idx_permission_approvals_agent_time",
            columns: &[
                IndexColumn {
                    name: Self::AGENT_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
        IndexDef {
            name: "idx_permission_approvals_conversation_time",
            columns: &[
                IndexColumn {
                    name: Self::CONVERSATION_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        Self::APPROVAL_ID,
        Self::STATUS,
        Self::AGENT_ID,
        Self::ACTION,
        Self::CONVERSATION_ID,
        Self::RUN_ID,
        Self::MESSAGE_ID,
        Self::TARGET_JSON,
        Self::SCOPE_JSON,
        Self::TRIGGER_JSON,
        Self::MATCHED_RULE_ID,
        Self::REASON,
        Self::CREATED_AT,
        Self::EXPIRES_AT,
        Self::RESOLVED_AT,
        Self::RESOLUTION,
    ];
    const SELECT_COLUMNS: &'static [&'static str] = &[
        Self::APPROVAL_ID,
        Self::STATUS,
        Self::AGENT_ID,
        Self::ACTION,
        Self::TARGET_JSON,
        Self::SCOPE_JSON,
        Self::TRIGGER_JSON,
        Self::MATCHED_RULE_ID,
        Self::REASON,
        Self::CREATED_AT,
        Self::EXPIRES_AT,
        Self::RESOLVED_AT,
        Self::RESOLUTION,
    ];
}

pub struct PermissionGrantsSchema;

impl PermissionGrantsSchema {
    pub const SEQ: &'static str = "seq";
    pub const GRANT_ID: &'static str = "grant_id";
    pub const APPROVAL_ID: &'static str = "approval_id";
    pub const AGENT_ID: &'static str = "agent_id";
    pub const MODE: &'static str = "mode";
    pub const REQUEST_JSON: &'static str = "request_json";
    pub const CONSUMED_AT: &'static str = "consumed_at";
    pub const EXPIRES_AT: &'static str = "expires_at";
    pub const CREATED_AT: &'static str = "created_at";
}

impl TableSchema for PermissionGrantsSchema {
    const NAME: &'static str = "permission_grants";
    const COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::SEQ,
            sql_type: SqlType::Integer,
            constraints: &["PRIMARY KEY", "AUTOINCREMENT"],
        },
        ColumnDef {
            name: Self::GRANT_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL", "UNIQUE"],
        },
        ColumnDef {
            name: Self::APPROVAL_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::AGENT_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::MODE,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::REQUEST_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::CONSUMED_AT,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::EXPIRES_AT,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::CREATED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
    ];
    const INDEXES: &'static [IndexDef] = &[IndexDef {
        name: "idx_permission_grants_agent_time",
        columns: &[
            IndexColumn {
                name: Self::AGENT_ID,
                descending: false,
            },
            IndexColumn {
                name: Self::CREATED_AT,
                descending: true,
            },
        ],
    }];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        Self::GRANT_ID,
        Self::APPROVAL_ID,
        Self::AGENT_ID,
        Self::MODE,
        Self::REQUEST_JSON,
        Self::CONSUMED_AT,
        Self::EXPIRES_AT,
        Self::CREATED_AT,
    ];
    const SELECT_COLUMNS: &'static [&'static str] = &[
        Self::GRANT_ID,
        Self::APPROVAL_ID,
        Self::AGENT_ID,
        Self::MODE,
        Self::REQUEST_JSON,
        Self::CONSUMED_AT,
        Self::EXPIRES_AT,
        Self::CREATED_AT,
    ];
}

pub fn table_statements() -> Vec<String> {
    vec![
        PermissionEventsSchema::create_table_statement(),
        PermissionApprovalsSchema::create_table_statement(),
        PermissionGrantsSchema::create_table_statement(),
    ]
}

pub fn index_statements() -> Vec<String> {
    let mut statements = Vec::new();
    statements.extend(PermissionEventsSchema::create_index_statements());
    statements.extend(PermissionApprovalsSchema::create_index_statements());
    statements.extend(PermissionGrantsSchema::create_index_statements());
    statements
}
