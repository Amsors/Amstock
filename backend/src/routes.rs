use std::collections::{HashMap, HashSet};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chrono::Utc;
use serde::Serialize;
use sqlx::{Row, Sqlite, Transaction};

use crate::{
    AppState,
    error::{AppError, Result},
    models::*,
    printing::{self, LabelStyle, PrintOutput},
    validation::{normalized_tag_a, validate_element},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/elements", get(search_elements).post(create_element))
        .route("/lookup", get(lookup_element))
        .route(
            "/elements/{serial}",
            get(get_element).put(update_element).delete(remove_element),
        )
        .route("/elements/{serial}/restore", post(restore_element))
        .route("/elements/{serial}/print", post(print_element_label))
        .route("/elements/{serial}/delete-preview", get(delete_preview))
        .route(
            "/elements/{serial}/image",
            put(put_image).delete(delete_image),
        )
        .route("/tree", get(get_tree))
        .route("/mappings/categories", get(list_categories))
        .route(
            "/mappings/categories/{tag_a}",
            put(put_category).delete(delete_category),
        )
        .route(
            "/mappings/categories/{tag_a}/mnemonics",
            get(list_mnemonics),
        )
        .route(
            "/mappings/categories/{tag_a}/mnemonics/{tag_b}",
            put(put_mnemonic).delete(delete_mnemonic),
        )
}

async fn print_element_label(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
    Json(request): Json<PrintRequest>,
) -> Result<Json<PrintOutput>> {
    let element = fetch_element(&state.pool, serial).await?;
    if element.deleted_at.is_some() {
        return Err(AppError::Conflict("不能打印已删除元素的标签".into()));
    }
    if matches!(request.style, LabelStyle::B1 | LabelStyle::B2) && element.kind != "container" {
        return Err(AppError::BadRequest("B1、B2 标签仅适用于容器".into()));
    }

    let mut children = if request.style.includes_children() {
        sqlx::query_as::<_, Element>(
            "SELECT * FROM elements WHERE parent_serial=? AND deleted_at IS NULL",
        )
        .bind(serial)
        .fetch_all(&state.pool)
        .await?
    } else {
        Vec::new()
    };
    printing::sort_children(&mut children);
    Ok(Json(
        printing::print(&state.print_config, &element, request.style, &children).await?,
    ))
}

fn view(element: Element) -> ElementView {
    element.into()
}

async fn fetch_element(pool: &sqlx::SqlitePool, serial: i64) -> Result<Element> {
    sqlx::query_as::<_, Element>("SELECT * FROM elements WHERE serial = ?")
        .bind(serial)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("未找到该序列号".into()))
}

async fn validate_parent(pool: &sqlx::SqlitePool, parent: Option<i64>) -> Result<()> {
    if let Some(parent) = parent {
        let row = sqlx::query("SELECT kind, deleted_at FROM elements WHERE serial = ?")
            .bind(parent)
            .fetch_optional(pool)
            .await?;
        match row {
            Some(row)
                if row.get::<String, _>("kind") == "container"
                    && row.get::<Option<String>, _>("deleted_at").is_none() =>
            {
                Ok(())
            }
            Some(_) => Err(AppError::BadRequest("父级必须是未删除的容器".into())),
            None => Err(AppError::BadRequest("父容器序列号不存在".into())),
        }
    } else {
        Ok(())
    }
}

async fn ensure_mapping(tx: &mut Transaction<'_, Sqlite>, tag_a: &str, tag_b: i64) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO category_mappings(tag_a, name) VALUES (?, NULL)")
        .bind(tag_a)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO mnemonic_mappings(tag_a, tag_b, name) VALUES (?, ?, NULL)")
        .bind(tag_a)
        .bind(tag_b)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn create_element(
    State(state): State<AppState>,
    Json(mut input): Json<CreateElement>,
) -> Result<(StatusCode, Json<ElementView>)> {
    let tag_a = validate_element(&input)?;
    validate_parent(&state.pool, input.parent_serial).await?;
    input.name = input.name.trim().to_string();
    input.description = input.description.trim().to_string();
    input.unit = input.unit.trim().to_string();
    let mut tx = state.pool.begin().await?;
    let serial: i64 = sqlx::query_scalar("UPDATE app_state SET next_serial = next_serial + 1 WHERE id = 1 AND next_serial < 1000000 RETURNING next_serial - 1")
        .fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::Conflict("六位序列号已用尽".into()))?;
    ensure_mapping(&mut tx, &tag_a, input.tag_b).await?;
    let element = sqlx::query_as::<_, Element>(
        r#"
        INSERT INTO elements(serial, kind, tag_a, tag_b, tag_c, name, description, quantity, unit, parent_serial)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *"#,
    )
    .bind(serial)
    .bind(&input.kind)
    .bind(&tag_a)
    .bind(input.tag_b)
    .bind(input.tag_c)
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.quantity)
    .bind(&input.unit)
    .bind(input.parent_serial)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(view(element))))
}

async fn search_elements(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<ElementView>>> {
    let rows = find_elements(&state.pool, &query).await?;
    Ok(Json(rows.into_iter().map(view).collect()))
}

/// 普通检索和 URL 只读检索共用这一查询入口；后续筛选条件统一在这里扩展。
async fn find_elements(pool: &sqlx::SqlitePool, query: &SearchQuery) -> Result<Vec<Element>> {
    let q = query.q.as_deref().unwrap_or_default().trim().to_string();
    let pattern = format!("%{q}%");
    let include_deleted = query.include_deleted.unwrap_or(false);
    Ok(sqlx::query_as::<_, Element>(
        r#"
        SELECT * FROM elements
        WHERE (? OR deleted_at IS NULL)
          AND (? = '' OR name LIKE ? ESCAPE '\' COLLATE NOCASE
            OR description LIKE ? ESCAPE '\' COLLATE NOCASE
            OR printf('%s-%02d-%02d-%06d', tag_a, tag_b, tag_c, serial) LIKE ?)
        ORDER BY deleted_at IS NOT NULL, updated_at DESC, serial DESC LIMIT 200"#,
    )
    .bind(include_deleted)
    .bind(&q)
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await?)
}

async fn get_element(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
) -> Result<Json<ElementView>> {
    Ok(Json(view(fetch_element(&state.pool, serial).await?)))
}

async fn lookup_element(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<ElementLookupView>>> {
    let matches = find_elements(&state.pool, &query).await?;
    if matches.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let all_elements = sqlx::query_as::<_, Element>("SELECT * FROM elements")
        .fetch_all(&state.pool)
        .await?;
    let by_serial: HashMap<i64, Element> = all_elements
        .into_iter()
        .map(|element| (element.serial, element))
        .collect();

    let mut results = Vec::with_capacity(matches.len());
    for element in matches {
        results.push(build_element_lookup(element, &by_serial)?);
    }
    Ok(Json(results))
}

fn build_element_lookup(
    element: Element,
    by_serial: &HashMap<i64, Element>,
) -> Result<ElementLookupView> {
    let mut path = vec![view(element.clone())];
    let mut parent_serial = element.parent_serial;
    let mut visited = HashSet::from([element.serial]);
    while let Some(parent) = parent_serial {
        if !visited.insert(parent) {
            return Err(AppError::Internal("检测到循环的父容器关系".into()));
        }
        let parent_element = by_serial
            .get(&parent)
            .ok_or_else(|| AppError::Internal("父容器数据不存在".into()))?;
        parent_serial = parent_element.parent_serial;
        path.push(view(parent_element.clone()));
    }
    path.reverse();
    Ok(ElementLookupView {
        element: view(element),
        path,
    })
}

async fn update_element(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
    Json(mut input): Json<UpdateElement>,
) -> Result<Json<ElementView>> {
    let tag_a = validate_element(&input)?;
    let current = fetch_element(&state.pool, serial).await?;
    if current.deleted_at.is_some() {
        return Err(AppError::Conflict("请先恢复该元素再编辑".into()));
    }
    if current.kind == "container" && input.kind == "item" {
        let has_children: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM elements WHERE parent_serial=? AND deleted_at IS NULL)",
        )
        .bind(serial)
        .fetch_one(&state.pool)
        .await?;
        if has_children {
            return Err(AppError::BadRequest(
                "非空容器不能改为物品，请先移动或删除其中的元素".into(),
            ));
        }
    }
    validate_parent(&state.pool, input.parent_serial).await?;
    if let Some(parent) = input.parent_serial {
        if parent == serial {
            return Err(AppError::BadRequest("元素不能放入自身".into()));
        }
        let cycle: bool = sqlx::query_scalar(r#"
            WITH RECURSIVE descendants(serial) AS (
              SELECT serial FROM elements WHERE parent_serial = ? AND deleted_at IS NULL
              UNION ALL SELECT e.serial FROM elements e JOIN descendants d ON e.parent_serial = d.serial WHERE e.deleted_at IS NULL
            ) SELECT EXISTS(SELECT 1 FROM descendants WHERE serial = ?)"#)
            .bind(serial).bind(parent).fetch_one(&state.pool).await?;
        if cycle {
            return Err(AppError::BadRequest("不能将容器移动到其后代容器中".into()));
        }
    }
    input.name = input.name.trim().to_string();
    input.description = input.description.trim().to_string();
    input.unit = input.unit.trim().to_string();
    let mut tx = state.pool.begin().await?;
    ensure_mapping(&mut tx, &tag_a, input.tag_b).await?;
    let element = sqlx::query_as::<_, Element>(r#"
        UPDATE elements SET kind=?, tag_a=?, tag_b=?, tag_c=?, name=?, description=?, quantity=?, unit=?, parent_serial=?, updated_at=CURRENT_TIMESTAMP
        WHERE serial=? RETURNING *"#)
        .bind(&input.kind).bind(&tag_a).bind(input.tag_b).bind(input.tag_c).bind(&input.name)
        .bind(&input.description).bind(input.quantity).bind(&input.unit).bind(input.parent_serial).bind(serial)
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(view(element)))
}

#[derive(Serialize)]
struct DeleteResult {
    deleted: usize,
}

async fn remove_element(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
    Json(request): Json<DeleteRequest>,
) -> Result<Json<DeleteResult>> {
    let element = fetch_element(&state.pool, serial).await?;
    if element.deleted_at.is_some() {
        return Err(AppError::Conflict("该元素已经删除".into()));
    }
    let child_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM elements WHERE parent_serial=? AND deleted_at IS NULL",
    )
    .bind(serial)
    .fetch_one(&state.pool)
    .await?;
    let now = Utc::now().to_rfc3339();
    let mut tx = state.pool.begin().await?;
    let mut deleted = 1usize;
    if child_count > 0 {
        let mode = request
            .mode
            .ok_or_else(|| AppError::BadRequest("非空容器必须明确选择子元素的处理方式".into()))?;
        match mode {
            DeleteMode::MoveToParent => {
                sqlx::query("UPDATE elements SET parent_serial=?, updated_at=CURRENT_TIMESTAMP WHERE parent_serial=? AND deleted_at IS NULL")
                    .bind(element.parent_serial).bind(serial).execute(&mut *tx).await?;
            }
            DeleteMode::MoveToContainer => {
                let target = request
                    .target_serial
                    .ok_or_else(|| AppError::BadRequest("需要提供目标容器序列号".into()))?;
                if target == serial {
                    return Err(AppError::BadRequest("目标容器不能是待删除容器自身".into()));
                }
                let valid_target: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM elements WHERE serial=? AND kind='container' AND deleted_at IS NULL)")
                    .bind(target).fetch_one(&mut *tx).await?;
                if !valid_target {
                    return Err(AppError::BadRequest("目标序列号不是可用容器".into()));
                }
                let is_descendant: bool = sqlx::query_scalar(r#"
                    WITH RECURSIVE descendants(serial) AS (
                      SELECT serial FROM elements WHERE parent_serial=? AND deleted_at IS NULL
                      UNION ALL SELECT e.serial FROM elements e JOIN descendants d ON e.parent_serial=d.serial WHERE e.deleted_at IS NULL
                    ) SELECT EXISTS(SELECT 1 FROM descendants WHERE serial=?)"#)
                    .bind(serial).bind(target).fetch_one(&mut *tx).await?;
                if is_descendant {
                    return Err(AppError::BadRequest(
                        "目标容器不能位于待删除容器内部".into(),
                    ));
                }
                sqlx::query("UPDATE elements SET parent_serial=?, updated_at=CURRENT_TIMESTAMP WHERE parent_serial=? AND deleted_at IS NULL")
                    .bind(target).bind(serial).execute(&mut *tx).await?;
            }
            DeleteMode::Cascade => {
                let affected = sqlx::query(r#"
                    WITH RECURSIVE descendants(serial) AS (
                      SELECT serial FROM elements WHERE parent_serial=? AND deleted_at IS NULL
                      UNION ALL SELECT e.serial FROM elements e JOIN descendants d ON e.parent_serial=d.serial WHERE e.deleted_at IS NULL
                    ) UPDATE elements SET deleted_at=?, updated_at=CURRENT_TIMESTAMP WHERE serial IN (SELECT serial FROM descendants)"#)
                    .bind(serial).bind(&now).execute(&mut *tx).await?;
                deleted += affected.rows_affected() as usize;
            }
        }
    }
    sqlx::query("UPDATE elements SET deleted_at=?, updated_at=CURRENT_TIMESTAMP WHERE serial=?")
        .bind(&now)
        .bind(serial)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(DeleteResult { deleted }))
}

#[derive(Serialize)]
struct PreviewEntry {
    depth: i64,
    #[serde(flatten)]
    element: ElementView,
}

async fn delete_preview(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
) -> Result<Json<Vec<PreviewEntry>>> {
    let element = fetch_element(&state.pool, serial).await?;
    if element.kind != "container" {
        return Ok(Json(Vec::new()));
    }
    let rows = sqlx::query(r#"
        WITH RECURSIVE tree(serial, depth, path) AS (
          SELECT serial, 1, printf('%06d', serial) FROM elements WHERE parent_serial=? AND deleted_at IS NULL
          UNION ALL
          SELECT e.serial, tree.depth+1, tree.path || '/' || printf('%06d', e.serial)
          FROM elements e JOIN tree ON e.parent_serial=tree.serial WHERE e.deleted_at IS NULL
        ) SELECT e.*, tree.depth FROM tree JOIN elements e ON e.serial=tree.serial ORDER BY tree.path"#)
        .bind(serial).fetch_all(&state.pool).await?;
    let mut result = Vec::new();
    for row in rows {
        let depth: i64 = row.try_get("depth")?;
        let element = Element {
            serial: row.try_get("serial")?,
            kind: row.try_get("kind")?,
            tag_a: row.try_get("tag_a")?,
            tag_b: row.try_get("tag_b")?,
            tag_c: row.try_get("tag_c")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            quantity: row.try_get("quantity")?,
            unit: row.try_get("unit")?,
            parent_serial: row.try_get("parent_serial")?,
            image_mime: row.try_get("image_mime")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            deleted_at: row.try_get("deleted_at")?,
        };
        result.push(PreviewEntry {
            depth,
            element: view(element),
        });
    }
    Ok(Json(result))
}

async fn restore_element(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
) -> Result<Json<ElementView>> {
    let current = fetch_element(&state.pool, serial).await?;
    if current.deleted_at.is_none() {
        return Err(AppError::Conflict("该元素未被删除".into()));
    }
    let parent_available = if let Some(parent) = current.parent_serial {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM elements WHERE serial=? AND kind='container' AND deleted_at IS NULL)")
            .bind(parent).fetch_one(&state.pool).await?
    } else {
        true
    };
    let parent = if parent_available {
        current.parent_serial
    } else {
        None
    };
    let mut tx = state.pool.begin().await?;
    ensure_mapping(&mut tx, &current.tag_a, current.tag_b).await?;
    let element = sqlx::query_as::<_, Element>("UPDATE elements SET deleted_at=NULL, parent_serial=?, updated_at=CURRENT_TIMESTAMP WHERE serial=? RETURNING *")
        .bind(parent).bind(serial).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(view(element)))
}

async fn get_tree(State(state): State<AppState>) -> Result<Json<Vec<TreeNode>>> {
    let elements = sqlx::query_as::<_, Element>("SELECT * FROM elements WHERE deleted_at IS NULL ORDER BY kind DESC, name COLLATE NOCASE, serial")
        .fetch_all(&state.pool).await?;
    let active: std::collections::HashSet<i64> = elements.iter().map(|e| e.serial).collect();
    let mut children: HashMap<Option<i64>, Vec<Element>> = HashMap::new();
    for element in elements {
        let parent = element.parent_serial.filter(|p| active.contains(p));
        children.entry(parent).or_default().push(element);
    }
    fn build(parent: Option<i64>, map: &mut HashMap<Option<i64>, Vec<Element>>) -> Vec<TreeNode> {
        map.remove(&parent)
            .unwrap_or_default()
            .into_iter()
            .map(|element| {
                let serial = element.serial;
                TreeNode {
                    element: view(element),
                    children: build(Some(serial), map),
                }
            })
            .collect()
    }
    Ok(Json(build(None, &mut children)))
}

async fn list_categories(State(state): State<AppState>) -> Result<Json<Vec<CategoryMapping>>> {
    Ok(Json(
        sqlx::query_as("SELECT * FROM category_mappings ORDER BY tag_a")
            .fetch_all(&state.pool)
            .await?,
    ))
}

async fn put_category(
    State(state): State<AppState>,
    Path(tag_a): Path<String>,
    Json(body): Json<MappingName>,
) -> Result<Json<CategoryMapping>> {
    let tag_a = normalized_tag_a(&tag_a)?;
    let name = body
        .name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let row = sqlx::query_as("INSERT INTO category_mappings(tag_a,name) VALUES(?,?) ON CONFLICT(tag_a) DO UPDATE SET name=excluded.name RETURNING *")
        .bind(tag_a).bind(name).fetch_one(&state.pool).await?;
    Ok(Json(row))
}

async fn delete_category(
    State(state): State<AppState>,
    Path(tag_a): Path<String>,
) -> Result<StatusCode> {
    let tag_a = normalized_tag_a(&tag_a)?;
    let used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM elements WHERE tag_a=? AND deleted_at IS NULL)",
    )
    .bind(&tag_a)
    .fetch_one(&state.pool)
    .await?;
    if used {
        return Err(AppError::Conflict(
            "该类别位仍被元素使用，只能清空名称，不能删除映射项".into(),
        ));
    }
    let affected = sqlx::query("DELETE FROM category_mappings WHERE tag_a=?")
        .bind(tag_a)
        .execute(&state.pool)
        .await?;
    if affected.rows_affected() == 0 {
        return Err(AppError::NotFound("映射不存在".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_mnemonics(
    State(state): State<AppState>,
    Path(tag_a): Path<String>,
) -> Result<Json<Vec<MnemonicMapping>>> {
    let tag_a = normalized_tag_a(&tag_a)?;
    Ok(Json(
        sqlx::query_as("SELECT * FROM mnemonic_mappings WHERE tag_a=? ORDER BY tag_b")
            .bind(tag_a)
            .fetch_all(&state.pool)
            .await?,
    ))
}

async fn put_mnemonic(
    State(state): State<AppState>,
    Path((tag_a, tag_b)): Path<(String, i64)>,
    Json(body): Json<MappingName>,
) -> Result<Json<MnemonicMapping>> {
    let tag_a = normalized_tag_a(&tag_a)?;
    if !(0..=99).contains(&tag_b) {
        return Err(AppError::BadRequest("助记位必须在 00 到 99 之间".into()));
    }
    let name = body
        .name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT OR IGNORE INTO category_mappings(tag_a,name) VALUES(?,NULL)")
        .bind(&tag_a)
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query_as("INSERT INTO mnemonic_mappings(tag_a,tag_b,name) VALUES(?,?,?) ON CONFLICT(tag_a,tag_b) DO UPDATE SET name=excluded.name RETURNING *")
        .bind(tag_a).bind(tag_b).bind(name).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(row))
}

async fn delete_mnemonic(
    State(state): State<AppState>,
    Path((tag_a, tag_b)): Path<(String, i64)>,
) -> Result<StatusCode> {
    let tag_a = normalized_tag_a(&tag_a)?;
    let used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM elements WHERE tag_a=? AND tag_b=? AND deleted_at IS NULL)",
    )
    .bind(&tag_a)
    .bind(tag_b)
    .fetch_one(&state.pool)
    .await?;
    if used {
        return Err(AppError::Conflict(
            "该助记位仍被元素使用，只能清空名称，不能删除映射项".into(),
        ));
    }
    let affected = sqlx::query("DELETE FROM mnemonic_mappings WHERE tag_a=? AND tag_b=?")
        .bind(tag_a)
        .bind(tag_b)
        .execute(&state.pool)
        .await?;
    if affected.rows_affected() == 0 {
        return Err(AppError::NotFound("映射不存在".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn put_image(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode> {
    let element = fetch_element(&state.pool, serial).await?;
    if element.deleted_at.is_some() {
        return Err(AppError::Conflict("不能为已删除元素上传图片".into()));
    }
    if body.is_empty() {
        return Err(AppError::BadRequest("图片内容为空".into()));
    }
    let mime = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !matches!(
        mime,
        "image/jpeg" | "image/png" | "image/webp" | "image/gif"
    ) {
        return Err(AppError::BadRequest(
            "仅支持 JPEG、PNG、WebP 或 GIF 图片".into(),
        ));
    }
    let path = state.image_dir.join(format!("{serial:06}"));
    let tmp = state.image_dir.join(format!(".{serial:06}.upload"));
    tokio::fs::write(&tmp, body).await?;
    tokio::fs::rename(&tmp, &path).await?;
    sqlx::query("UPDATE elements SET image_mime=?, updated_at=CURRENT_TIMESTAMP WHERE serial=?")
        .bind(mime)
        .bind(serial)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_image(
    State(state): State<AppState>,
    Path(serial): Path<i64>,
) -> Result<StatusCode> {
    fetch_element(&state.pool, serial).await?;
    let path = state.image_dir.join(format!("{serial:06}"));
    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    sqlx::query("UPDATE elements SET image_mime=NULL, updated_at=CURRENT_TIMESTAMP WHERE serial=?")
        .bind(serial)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_image(State(state): State<AppState>, Path(serial): Path<i64>) -> Result<Response> {
    let mime: Option<String> = sqlx::query_scalar("SELECT image_mime FROM elements WHERE serial=?")
        .bind(serial)
        .fetch_optional(&state.pool)
        .await?
        .flatten();
    let mime = mime.ok_or_else(|| AppError::NotFound("该元素没有图片".into()))?;
    let bytes = tokio::fs::read(state.image_dir.join(format!("{serial:06}")))
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound("图片文件不存在".into())
            } else {
                e.into()
            }
        })?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "no-cache".into()),
        ],
        bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(serial: i64, parent_serial: Option<i64>, name: &str) -> Element {
        Element {
            serial,
            kind: if name == "物品" {
                "item"
            } else {
                "container"
            }
            .into(),
            tag_a: "A".into(),
            tag_b: 0,
            tag_c: 0,
            name: name.into(),
            description: String::new(),
            quantity: 1.0,
            unit: String::new(),
            parent_serial,
            image_mime: None,
            created_at: "2026-01-01 00:00:00".into(),
            updated_at: "2026-01-01 00:00:00".into(),
            deleted_at: None,
        }
    }

    #[test]
    fn lookup_path_runs_from_outermost_container_to_element() {
        let outer = element(1, None, "外箱");
        let inner = element(2, Some(1), "内盒");
        let item = element(3, Some(2), "物品");
        let by_serial = [outer, inner, item.clone()]
            .into_iter()
            .map(|entry| (entry.serial, entry))
            .collect();

        let result = build_element_lookup(item, &by_serial).unwrap();
        assert_eq!(
            result
                .path
                .iter()
                .map(|entry| entry.element.serial)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }
}
