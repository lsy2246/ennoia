use rusqlite::{types::Value, Connection};

#[derive(Debug, Clone, Copy)]
pub enum FilterOperator {
    Eq,
    Lte,
    IsNull,
}

#[derive(Debug, Clone)]
struct Filter {
    column: &'static str,
    operator: FilterOperator,
    value: Option<Value>,
}

#[derive(Debug, Clone)]
struct Assignment {
    column: &'static str,
    value: Value,
}

#[derive(Debug, Clone, Copy)]
struct OrderBy {
    column: &'static str,
    descending: bool,
}

#[derive(Debug, Clone)]
pub struct SelectQuery {
    table: &'static str,
    columns: &'static [&'static str],
    filters: Vec<Filter>,
    order_by: Option<OrderBy>,
    limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UpdateQuery {
    table: &'static str,
    assignments: Vec<Assignment>,
    filters: Vec<Filter>,
}

#[derive(Debug)]
pub struct PreparedStatement {
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
            value: Some(value),
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

    pub fn build(self) -> PreparedStatement {
        let mut sql = format!("SELECT {} FROM {}", self.columns.join(", "), self.table);
        let mut params = Vec::with_capacity(self.filters.len() + usize::from(self.limit.is_some()));
        append_filters(&mut sql, &mut params, self.filters);
        if let Some(order_by) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(order_by.column);
            sql.push_str(if order_by.descending { " DESC" } else { " ASC" });
        }
        if let Some(limit) = self.limit {
            sql.push_str(" LIMIT ?");
            params.push(limit.into());
        }
        PreparedStatement { sql, params }
    }
}

impl UpdateQuery {
    pub fn new(table: &'static str) -> Self {
        Self {
            table,
            assignments: Vec::new(),
            filters: Vec::new(),
        }
    }

    pub fn push_assignment(&mut self, column: &'static str, value: Value) {
        self.assignments.push(Assignment { column, value });
    }

    pub fn push_filter(&mut self, column: &'static str, operator: FilterOperator, value: Value) {
        self.filters.push(Filter {
            column,
            operator,
            value: Some(value),
        });
    }

    pub fn push_null_filter(&mut self, column: &'static str) {
        self.filters.push(Filter {
            column,
            operator: FilterOperator::IsNull,
            value: None,
        });
    }

    pub fn build(self) -> PreparedStatement {
        let mut sql = format!("UPDATE {} SET ", self.table);
        let mut params = Vec::with_capacity(self.assignments.len() + self.filters.len());
        for (index, assignment) in self.assignments.into_iter().enumerate() {
            if index > 0 {
                sql.push_str(", ");
            }
            sql.push_str(assignment.column);
            sql.push_str(" = ?");
            params.push(assignment.value);
        }
        append_filters(&mut sql, &mut params, self.filters);
        PreparedStatement { sql, params }
    }
}

impl PreparedStatement {
    pub fn prepare<'a>(
        &self,
        connection: &'a Connection,
    ) -> std::io::Result<rusqlite::Statement<'a>> {
        connection.prepare(&self.sql).map_err(std::io::Error::other)
    }

    pub fn execute(&self, connection: &Connection) -> std::io::Result<usize> {
        connection
            .execute(&self.sql, rusqlite::params_from_iter(self.params.iter()))
            .map_err(std::io::Error::other)
    }
}

fn append_filters(sql: &mut String, params: &mut Vec<Value>, filters: Vec<Filter>) {
    if filters.is_empty() {
        return;
    }

    sql.push_str(" WHERE ");
    for (index, filter) in filters.into_iter().enumerate() {
        if index > 0 {
            sql.push_str(" AND ");
        }
        match filter.operator {
            FilterOperator::Eq => {
                sql.push_str(filter.column);
                sql.push_str(" = ?");
            }
            FilterOperator::Lte => {
                sql.push_str(filter.column);
                sql.push_str(" <= ?");
            }
            FilterOperator::IsNull => {
                sql.push_str(filter.column);
                sql.push_str(" IS NULL");
            }
        }
        if let Some(value) = filter.value {
            params.push(value);
        }
    }
}
