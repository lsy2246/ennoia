use super::query::ColumnCount;

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
            .map(|column| {
                let mut definition = format!("{} {}", column.name, column.sql_type.render());
                if !column.constraints.is_empty() {
                    definition.push(' ');
                    definition.push_str(&column.constraints.join(" "));
                }
                definition
            })
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

    fn count_statement(count: ColumnCount) -> String {
        match count {
            ColumnCount::All => format!("SELECT COUNT(*) FROM {}", Self::NAME),
            ColumnCount::Distinct(column) => {
                format!("SELECT COUNT(DISTINCT {}) FROM {}", column, Self::NAME)
            }
        }
    }
}

pub struct LogsSchema;

impl LogsSchema {
    pub const SEQ: &'static str = "seq";
    pub const ID: &'static str = "id";
    pub const EVENT: &'static str = "event";
    pub const LEVEL: &'static str = "level";
    pub const COMPONENT: &'static str = "component";
    pub const SOURCE_KIND: &'static str = "source_kind";
    pub const SOURCE_ID: &'static str = "source_id";
    pub const REQUEST_ID: &'static str = "request_id";
    pub const TRACE_ID: &'static str = "trace_id";
    pub const SPAN_ID: &'static str = "span_id";
    pub const PARENT_SPAN_ID: &'static str = "parent_span_id";
    pub const MESSAGE: &'static str = "message";
    pub const ATTRIBUTES_JSON: &'static str = "attributes_json";
    pub const CREATED_AT: &'static str = "created_at";

    pub fn select_by_id_statement() -> String {
        format!(
            "SELECT {} FROM {} WHERE {} = ?1",
            <Self as TableSchema>::SELECT_COLUMNS.join(", "),
            <Self as TableSchema>::NAME,
            Self::ID,
        )
    }
}

impl TableSchema for LogsSchema {
    const NAME: &'static str = "logs";
    const COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::SEQ,
            sql_type: SqlType::Integer,
            constraints: &["PRIMARY KEY", "AUTOINCREMENT"],
        },
        ColumnDef {
            name: Self::ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL", "UNIQUE"],
        },
        ColumnDef {
            name: Self::EVENT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::LEVEL,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::COMPONENT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SOURCE_KIND,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SOURCE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::REQUEST_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::TRACE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::SPAN_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::PARENT_SPAN_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::MESSAGE,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::ATTRIBUTES_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::CREATED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
    ];
    const INDEXES: &'static [IndexDef] = &[
        IndexDef {
            name: "idx_logs_event_time",
            columns: &[
                IndexColumn {
                    name: Self::EVENT,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
        IndexDef {
            name: "idx_logs_component_time",
            columns: &[
                IndexColumn {
                    name: Self::COMPONENT,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
        IndexDef {
            name: "idx_logs_source_time",
            columns: &[
                IndexColumn {
                    name: Self::SOURCE_KIND,
                    descending: false,
                },
                IndexColumn {
                    name: Self::SOURCE_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::CREATED_AT,
                    descending: true,
                },
            ],
        },
        IndexDef {
            name: "idx_logs_trace_time",
            columns: &[
                IndexColumn {
                    name: Self::TRACE_ID,
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
        Self::ID,
        Self::EVENT,
        Self::LEVEL,
        Self::COMPONENT,
        Self::SOURCE_KIND,
        Self::SOURCE_ID,
        Self::REQUEST_ID,
        Self::TRACE_ID,
        Self::SPAN_ID,
        Self::PARENT_SPAN_ID,
        Self::MESSAGE,
        Self::ATTRIBUTES_JSON,
        Self::CREATED_AT,
    ];
    const SELECT_COLUMNS: &'static [&'static str] = &[
        Self::SEQ,
        Self::ID,
        Self::EVENT,
        Self::LEVEL,
        Self::COMPONENT,
        Self::SOURCE_KIND,
        Self::SOURCE_ID,
        Self::REQUEST_ID,
        Self::TRACE_ID,
        Self::SPAN_ID,
        Self::PARENT_SPAN_ID,
        Self::MESSAGE,
        Self::ATTRIBUTES_JSON,
        Self::CREATED_AT,
    ];
}

pub struct SpansSchema;

impl SpansSchema {
    pub const SEQ: &'static str = "seq";
    pub const ID: &'static str = "id";
    pub const TRACE_ID: &'static str = "trace_id";
    pub const SPAN_ID: &'static str = "span_id";
    pub const PARENT_SPAN_ID: &'static str = "parent_span_id";
    pub const REQUEST_ID: &'static str = "request_id";
    pub const SAMPLED: &'static str = "sampled";
    pub const SOURCE: &'static str = "source";
    pub const KIND: &'static str = "kind";
    pub const NAME_COL: &'static str = "name";
    pub const COMPONENT: &'static str = "component";
    pub const SOURCE_KIND: &'static str = "source_kind";
    pub const SOURCE_ID: &'static str = "source_id";
    pub const STATUS: &'static str = "status";
    pub const ATTRIBUTES_JSON: &'static str = "attributes_json";
    pub const STARTED_AT: &'static str = "started_at";
    pub const ENDED_AT: &'static str = "ended_at";
    pub const DURATION_MS: &'static str = "duration_ms";
}

impl TableSchema for SpansSchema {
    const NAME: &'static str = "spans";
    const COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::SEQ,
            sql_type: SqlType::Integer,
            constraints: &["PRIMARY KEY", "AUTOINCREMENT"],
        },
        ColumnDef {
            name: Self::ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL", "UNIQUE"],
        },
        ColumnDef {
            name: Self::TRACE_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SPAN_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::PARENT_SPAN_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::REQUEST_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SAMPLED,
            sql_type: SqlType::Integer,
            constraints: &["NOT NULL", "DEFAULT 1"],
        },
        ColumnDef {
            name: Self::SOURCE,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::KIND,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::NAME_COL,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::COMPONENT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SOURCE_KIND,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SOURCE_ID,
            sql_type: SqlType::Text,
            constraints: &[],
        },
        ColumnDef {
            name: Self::STATUS,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::ATTRIBUTES_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::STARTED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::ENDED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::DURATION_MS,
            sql_type: SqlType::Integer,
            constraints: &["NOT NULL"],
        },
    ];
    const INDEXES: &'static [IndexDef] = &[
        IndexDef {
            name: "idx_spans_trace_seq",
            columns: &[
                IndexColumn {
                    name: Self::TRACE_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::SEQ,
                    descending: false,
                },
            ],
        },
        IndexDef {
            name: "idx_spans_request_seq",
            columns: &[
                IndexColumn {
                    name: Self::REQUEST_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::SEQ,
                    descending: false,
                },
            ],
        },
        IndexDef {
            name: "idx_spans_component_time",
            columns: &[
                IndexColumn {
                    name: Self::COMPONENT,
                    descending: false,
                },
                IndexColumn {
                    name: Self::ENDED_AT,
                    descending: true,
                },
            ],
        },
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        Self::ID,
        Self::TRACE_ID,
        Self::SPAN_ID,
        Self::PARENT_SPAN_ID,
        Self::REQUEST_ID,
        Self::SAMPLED,
        Self::SOURCE,
        Self::KIND,
        Self::NAME_COL,
        Self::COMPONENT,
        Self::SOURCE_KIND,
        Self::SOURCE_ID,
        Self::STATUS,
        Self::ATTRIBUTES_JSON,
        Self::STARTED_AT,
        Self::ENDED_AT,
        Self::DURATION_MS,
    ];
    const SELECT_COLUMNS: &'static [&'static str] = &[
        Self::SEQ,
        Self::ID,
        Self::TRACE_ID,
        Self::SPAN_ID,
        Self::PARENT_SPAN_ID,
        Self::REQUEST_ID,
        Self::SAMPLED,
        Self::SOURCE,
        Self::KIND,
        Self::NAME_COL,
        Self::COMPONENT,
        Self::SOURCE_KIND,
        Self::SOURCE_ID,
        Self::STATUS,
        Self::ATTRIBUTES_JSON,
        Self::STARTED_AT,
        Self::ENDED_AT,
        Self::DURATION_MS,
    ];
}

pub struct SpanLinksSchema;

impl SpanLinksSchema {
    pub const SEQ: &'static str = "seq";
    pub const ID: &'static str = "id";
    pub const TRACE_ID: &'static str = "trace_id";
    pub const SPAN_ID: &'static str = "span_id";
    pub const LINKED_TRACE_ID: &'static str = "linked_trace_id";
    pub const LINKED_SPAN_ID: &'static str = "linked_span_id";
    pub const LINK_TYPE: &'static str = "link_type";
    pub const ATTRIBUTES_JSON: &'static str = "attributes_json";
    pub const CREATED_AT: &'static str = "created_at";
}

impl TableSchema for SpanLinksSchema {
    const NAME: &'static str = "span_links";
    const COLUMNS: &'static [ColumnDef] = &[
        ColumnDef {
            name: Self::SEQ,
            sql_type: SqlType::Integer,
            constraints: &["PRIMARY KEY", "AUTOINCREMENT"],
        },
        ColumnDef {
            name: Self::ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL", "UNIQUE"],
        },
        ColumnDef {
            name: Self::TRACE_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::SPAN_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::LINKED_TRACE_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::LINKED_SPAN_ID,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::LINK_TYPE,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::ATTRIBUTES_JSON,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
        ColumnDef {
            name: Self::CREATED_AT,
            sql_type: SqlType::Text,
            constraints: &["NOT NULL"],
        },
    ];
    const INDEXES: &'static [IndexDef] = &[
        IndexDef {
            name: "idx_span_links_trace_seq",
            columns: &[
                IndexColumn {
                    name: Self::TRACE_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::SEQ,
                    descending: false,
                },
            ],
        },
        IndexDef {
            name: "idx_span_links_span",
            columns: &[
                IndexColumn {
                    name: Self::SPAN_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::LINKED_SPAN_ID,
                    descending: false,
                },
                IndexColumn {
                    name: Self::LINK_TYPE,
                    descending: false,
                },
            ],
        },
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        Self::ID,
        Self::TRACE_ID,
        Self::SPAN_ID,
        Self::LINKED_TRACE_ID,
        Self::LINKED_SPAN_ID,
        Self::LINK_TYPE,
        Self::ATTRIBUTES_JSON,
        Self::CREATED_AT,
    ];
    const SELECT_COLUMNS: &'static [&'static str] = &[
        Self::SEQ,
        Self::ID,
        Self::TRACE_ID,
        Self::SPAN_ID,
        Self::LINKED_TRACE_ID,
        Self::LINKED_SPAN_ID,
        Self::LINK_TYPE,
        Self::ATTRIBUTES_JSON,
        Self::CREATED_AT,
    ];
}

pub fn schema_statements() -> Vec<String> {
    let mut statements = Vec::new();
    statements.push(LogsSchema::create_table_statement());
    statements.extend(LogsSchema::create_index_statements());
    statements.push(SpansSchema::create_table_statement());
    statements.extend(SpansSchema::create_index_statements());
    statements.push(SpanLinksSchema::create_table_statement());
    statements.extend(SpanLinksSchema::create_index_statements());
    statements
}
