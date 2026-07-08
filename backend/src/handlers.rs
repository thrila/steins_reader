use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusqlite::params;
use std::sync::Arc;

use crate::db::Db;
use crate::kalman;
use crate::models::*;

type AppState = Arc<Db>;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn row_to_skill(row: &rusqlite::Row) -> rusqlite::Result<Skill> {
    Ok(Skill {
        id: row.get(0)?,
        name: row.get(1)?,
        mean: row.get(2)?,
        variance: row.get(3)?,
        process_noise: row.get(4)?,
        position: row.get(5)?,
        created_at: row.get(6)?,
        last_updated: row.get(7)?,
    })
}

pub async fn list_skills(State(db): State<AppState>) -> Result<Json<Vec<Skill>>, StatusCode> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, mean, variance, process_noise, position, created_at, last_updated FROM skills ORDER BY position ASC")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let skills = stmt
        .query_map([], row_to_skill)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(skills))
}

pub async fn create_skill(
    State(db): State<AppState>,
    Json(body): Json<CreateSkill>,
) -> Result<Json<Skill>, StatusCode> {
    let conn = db.0.lock().unwrap();
    let position: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM skills",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let id = uuid::Uuid::new_v4().to_string();
    let t = now();
    let mean = body.initial_mean.unwrap_or(50.0).clamp(0.0, 100.0);
    conn.execute(
        "INSERT INTO skills (id, name, mean, variance, process_noise, position, created_at, last_updated)
         VALUES (?1, ?2, ?3, 400.0, 1.5, ?4, ?5, ?5)",
        params![id, body.name, mean, position, t],
    )
    .map_err(|_| StatusCode::CONFLICT)?;

    let skill = conn
        .query_row(
            "SELECT id, name, mean, variance, process_noise, position, created_at, last_updated FROM skills WHERE id = ?1",
            params![id],
            row_to_skill,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(skill))
}

pub async fn update_skill(
    State(db): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSkill>,
) -> Result<Json<Skill>, StatusCode> {
    let conn = db.0.lock().unwrap();
    if let Some(name) = &body.name {
        conn.execute(
            "UPDATE skills SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
        .map_err(|_| StatusCode::CONFLICT)?;
    }
    if let Some(mean) = body.mean {
        conn.execute(
            "UPDATE skills SET mean = ?1 WHERE id = ?2",
            params![mean.clamp(0.0, 100.0), id],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(variance) = body.variance {
        conn.execute(
            "UPDATE skills SET variance = ?1 WHERE id = ?2",
            params![variance, id],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(pn) = body.process_noise {
        conn.execute(
            "UPDATE skills SET process_noise = ?1 WHERE id = ?2",
            params![pn, id],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(position) = body.position {
        conn.execute(
            "UPDATE skills SET position = ?1 WHERE id = ?2",
            params![position, id],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    conn.execute(
        "UPDATE skills SET last_updated = ?1 WHERE id = ?2",
        params![now(), id],
    )
    .ok();

    let skill = conn
        .query_row(
            "SELECT id, name, mean, variance, process_noise, position, created_at, last_updated FROM skills WHERE id = ?1",
            params![id],
            row_to_skill,
        )
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(skill))
}

pub async fn delete_skill(State(db): State<AppState>, Path(id): Path<String>) -> StatusCode {
    let conn = db.0.lock().unwrap();
    match conn.execute("DELETE FROM skills WHERE id = ?1", params![id]) {
        Ok(0) => StatusCode::NOT_FOUND,
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn observe_skill(
    State(db): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ObserveRequest>,
) -> Result<Json<ObserveResponse>, StatusCode> {
    let conn = db.0.lock().unwrap();

    let (mean, variance, process_noise, last_updated): (f64, f64, f64, i64) = conn
        .query_row(
            "SELECT mean, variance, process_noise, last_updated FROM skills WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let t = now();
    let dt_days = ((t - last_updated).max(0) as f64) / 86400.0;
    let noise = body
        .measurement_noise
        .unwrap_or_else(|| body.source.default_noise());

    let result =
        kalman::predict_and_update(mean, variance, process_noise, dt_days, body.score, noise);

    conn.execute(
        "UPDATE skills SET mean = ?1, variance = ?2, last_updated = ?3 WHERE id = ?4",
        params![result.mean, result.variance, t, id],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let obs_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO observations (id, skill_id, score, measurement_noise, source, note, resulting_mean, resulting_variance, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            obs_id,
            id,
            body.score,
            noise,
            body.source.as_str(),
            body.note,
            result.mean,
            result.variance,
            t
        ],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let skill = conn
        .query_row(
            "SELECT id, name, mean, variance, process_noise, position, created_at, last_updated FROM skills WHERE id = ?1",
            params![id],
            row_to_skill,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let observation = Observation {
        id: obs_id,
        skill_id: id,
        score: body.score,
        measurement_noise: noise,
        source: body.source.as_str().to_string(),
        note: body.note,
        resulting_mean: result.mean,
        resulting_variance: result.variance,
        created_at: t,
    };

    Ok(Json(ObserveResponse {
        skill,
        observation,
        kalman_gain: result.gain,
    }))
}

pub async fn skill_history(
    State(db): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Observation>>, StatusCode> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, skill_id, score, measurement_noise, source, note, resulting_mean, resulting_variance, created_at
             FROM observations WHERE skill_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = stmt
        .query_map(params![id], |r| {
            Ok(Observation {
                id: r.get(0)?,
                skill_id: r.get(1)?,
                score: r.get(2)?,
                measurement_noise: r.get(3)?,
                source: r.get(4)?,
                note: r.get(5)?,
                resulting_mean: r.get(6)?,
                resulting_variance: r.get(7)?,
                created_at: r.get(8)?,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(rows))
}
