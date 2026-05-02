use rusqlite::{types::Value, Connection};

#[derive(Debug, Clone, Copy)]
pub enum FilterOperator {
    Eq,
    EqIgnoreCase,
    Lt,
    Gt,
}

#[derive(Debug, Clone)]
struct Filter {
    column: &'static str,
    operator: FilterOperator,
    value: Value,
}

#[derive(Debug, Clone, Copy)]
struct OrderBy {
    column: &'static str,
    descending: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ColumnCount {
    All,
    Distinct(&'static str),
}

#[derive(Debug, Clone)]
pub struct SelectQuery {
    table: &'static str,
    columns: &'static [&'static str],
    filters: Vec<Filter>,
    order_by: Option<OrderBy>,
    limit: Option<i64>,
}

#[derive(Debug)]
pub struct PreparedSelect {
    pub sql: String,
    pub params: Vec<Value>,
}

impl SelectQuery {
    pub fn new(table: &'static str, columns: &'static [&'static str]) -> Self {
        Self {
            table,
            columns,
            filters: Vec::new(),
            order_by: None,
            limit: None,
        }
    }

    pub fn push_filter(&mut self, column: &'static str, operator: FilterOperator, value: Value) {
        self.filters.push(Filter {
            column,
            operator,
            value,
        });
    }

    pub fn order_by(mut self, column: &'static str, descending: bool) -> Self {
        self.order_by = Some(OrderBy { column, descending });
        self
    }

    pub fn limit(mut self, value: i64) -> Self {
        self.limit = Some(value.max(1));
        self
    }

    pub fn build(self) -> PreparedSelect {
        let mut sql = format!("SELECT {} FROM {}", self.columns.join(", "), self.table);
        let mut params = Vec::with_capacity(self.filters.len() + usize::from(self.limit.is_some()));

        if !self.filters.is_empty() {
            sql.push_str(" WHERE ");
            for (index, filter) in self.filters.into_iter().enumerate() {
                if index > 0 {
                    sql.push_str(" AND ");
                }
                match filter.operator {
                    FilterOperator::Eq => sql.push_str(&format!("{} = ?", filter.column)),
                    FilterOperator::EqIgnoreCase => {
                        sql.push_str(&format!("lower({}) = lower(?)", filter.column));
                    }
                    FilterOperator::Lt => sql.push_str(&format!("{} < ?", filter.column)),
                    FilterOperator::Gt => sql.push_str(&format!("{} > ?", filter.column)),
                }
                params.push(filter.value);
            }
        }

        if let Some(order_by) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(order_by.column);
            sql.push_str(if order_by.descending { " DESC" } else { " ASC" });
        }

        if let Some(limit) = self.limit {
            sql.push_str(" LIMIT ?");
            params.push(limit.into());
        }

        PreparedSelect { sql, params }
    }
}

impl PreparedSelect {
    pub fn prepare<'a>(
        &self,
        connection: &'a Connection,
    ) -> std::io::Result<rusqlite::Statement<'a>> {
        connection.prepare(&self.sql).map_err(std::io::Error::other)
    }
}
