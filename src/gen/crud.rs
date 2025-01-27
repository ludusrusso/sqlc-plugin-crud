use std::vec;

use super::conf::{Config, TableConfig};
use super::data::{Column, Named, Table};

pub fn crud(t: &Table, opts: &Config) -> Option<String> {
    let name = t.name();
    let conf = opts.tables.iter().find(|c| c.table == name)?;

    let create = create_query(t)?;
    let update = update_query(t, conf)?;
    let get = get_query(t, conf)?;
    let list = list_query(t, conf)?;
    let cnt = count_list_query(t, conf)?;
    let delete = delete_query(t, conf)?;

    let head = include_str!("./head.txt");

    let res = format!(
        "{}\n\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
        head, create, update, get, list, cnt, delete
    );
    Some(res)
}

fn create_query(table: &Table) -> Option<String> {
    let cols = get_create_cols(&table.columns);

    let col_list = list_columns_with_prefix(&cols, "");
    let params_list = list_columns_with_prefix(&cols, "@");

    let res = format!(
        "-- name: Create{} :one \nINSERT INTO {} ({}) VALUES ({}) RETURNING *;",
        table.singular(),
        table.name(),
        col_list,
        params_list
    );

    Some(res)
}

fn update_query(t: &Table, conf: &TableConfig) -> Option<String> {
    let cols = get_update_cols(&t.columns, conf);
    let upds = cols
        .iter()
        .map(|c| get_update_sql(c))
        .collect::<Vec<String>>()
        .join(",\n  ");

    let res = format!(
        "-- name: Update{} :one \nUPDATE {} SET \n  {} \n  WHERE {} RETURNING *;",
        t.singular(),
        t.name(),
        upds,
        select_one(conf)
    );

    Some(res)
}

fn delete_query(t: &Table, conf: &TableConfig) -> Option<String> {
    let res = format!(
        "-- name: Delete{} :one \nDELETE FROM {} WHERE {} RETURNING *;",
        t.singular(),
        t.name(),
        select_one(conf)
    );

    Some(res)
}

fn list_query(t: &Table, conf: &TableConfig) -> Option<String> {
    let tnt = select_tenant(conf);

    let res = if tnt.is_empty() {
        format!(
            "-- name: List{} :many \nSELECT * FROM {} LIMIT @take OFFSET @skip;",
            t.plural(),
            t.name(),
        )
    } else {
        format!(
            "-- name: List{} :many \nSELECT * FROM {} WHERE {} LIMIT @take OFFSET @skip;",
            t.plural(),
            t.name(),
            tnt
        )
    };

    Some(res)
}

fn get_query(t: &Table, conf: &TableConfig) -> Option<String> {
    let tnt = select_one(conf);

    let res = format!(
        "-- name: Get{} :one \nSELECT * FROM {} WHERE {};",
        t.singular(),
        t.name(),
        tnt
    );

    Some(res)
}

fn count_list_query(t: &Table, conf: &TableConfig) -> Option<String> {
    let tnt = select_tenant(conf);

    let res = if tnt.is_empty() {
        format!(
            "-- name: CountList{} :one \nSELECT COUNT(*) FROM {};",
            t.plural(),
            t.name(),
        )
    } else {
        format!(
            "-- name: CountList{} :one \nSELECT COUNT(*) FROM {} WHERE {};",
            t.plural(),
            t.name(),
            tnt
        )
    };

    Some(res)
}

fn select_one(conf: &TableConfig) -> String {
    let mut ids: Vec<&String> = vec![];
    ids.extend(&conf.id_cols);
    ids.extend(&conf.tenants_cols);

    ids.iter()
        .map(|id| format!("{} = @{}", id, id))
        .collect::<Vec<String>>()
        .join(" AND ")
}

fn select_tenant(conf: &TableConfig) -> String {
    let mut ids: Vec<&String> = vec![];
    ids.extend(&conf.tenants_cols);

    ids.iter()
        .map(|id| format!("{} = @{}", id, id))
        .collect::<Vec<String>>()
        .join(" AND ")
}

fn get_create_cols(cols: &Vec<Column>) -> Vec<Column> {
    cols.iter()
        .filter(|col| col.name != "updated_at" && col.name != "created_at")
        .map(|c| c.clone())
        .collect::<Vec<Column>>()
}

fn get_update_cols(cols: &Vec<Column>, conf: &TableConfig) -> Vec<Column> {
    cols.iter()
        .filter(|col| {
            col.name != "created_at"
                && !conf.id_cols.contains(&col.name)
                && !conf.tenants_cols.contains(&col.name)
        })
        .map(|c| c.clone())
        .collect::<Vec<Column>>()
}

fn list_columns_with_prefix(cols: &Vec<Column>, prefix: &str) -> String {
    cols.iter()
        .map(|col| format!("{}{}", prefix, col.name).to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

fn get_update_sql(col: &Column) -> String {
    if col.name == "updated_at" {
        return format!("{} = now()", col.name);
    }

    format!(
        "{} = COALESCE(sqlc.narg({})::{}, {})",
        col.name,
        col.name,
        col.r#type.to_owned().unwrap().name,
        col.name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_columns() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let cols = table.columns;
        let prefix = "@";

        let res = list_columns_with_prefix(&cols, prefix);
        assert_eq!(res, "@id, @name, @bio, @created_at, @updated_at");

        let res = list_columns_with_prefix(&cols, "");
        assert_eq!(res, "id, name, bio, created_at, updated_at");
    }

    #[test]
    fn test_create_query() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let res = create_query(&table);
        assert_eq!(res, Some("-- name: CreateAuthor :one \nINSERT INTO authors (id, name, bio) VALUES (@id, @name, @bio) RETURNING *;".to_string()));
    }

    #[test]
    fn test_delete_query() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let res = delete_query(&table, &TableConfig::default());
        assert_eq!(
            res,
            Some(
                "-- name: DeleteAuthor :one \nDELETE FROM authors WHERE id = @id RETURNING *;"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_list_query() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let res = list_query(&table, &TableConfig::default());

        assert_eq!(
            res,
            Some(
                "-- name: ListAuthors :many \nSELECT * FROM authors LIMIT @take OFFSET @skip;"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_get_query() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let res = get_query(&table, &TableConfig::default());

        assert_eq!(
            res,
            Some("-- name: GetAuthor :one \nSELECT * FROM authors WHERE id = @id;".to_string())
        );
    }

    #[test]
    fn test_count_list_query() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let res = count_list_query(&table, &TableConfig::default());
        assert_eq!(
            res,
            Some("-- name: CountListAuthors :one \nSELECT COUNT(*) FROM authors;".to_string())
        );
    }

    #[test]
    fn test_update_query() {
        let table_json = include_str!("./tests/table.json");
        let table: Table = serde_json::from_str(table_json).unwrap();

        let res = update_query(&table, &TableConfig::default());

        assert_eq!(
            res,
            Some(
                "-- name: UpdateAuthor :one \nUPDATE authors SET \n  name = COALESCE(sqlc.narg(name)::text, name),\n  bio = COALESCE(sqlc.narg(bio)::text, bio),\n  updated_at = now() \n  WHERE id = @id RETURNING *;"
                    .to_string()
            )
        );
    }
}
