use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub mean: f64,
    pub variance: f64,
    pub process_noise: f64,
    pub position: i64,
    pub created_at: i64,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Observation {
    pub id: String,
    pub skill_id: String,
    pub score: f64,
    pub measurement_noise: f64,
    pub source: String,
    pub note: Option<String>,
    pub resulting_mean: f64,
    pub resulting_variance: f64,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSkill {
    pub name: String,
    #[serde(default)]
    pub initial_mean: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSkill {
    pub name: Option<String>,
    pub mean: Option<f64>,
    pub variance: Option<f64>,
    pub process_noise: Option<f64>,
    pub position: Option<i64>,
}

/// Known measurement-noise presets per observation source.
/// Lower noise => the system trusts the observation more => bigger Kalman gain.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    StandardizedTest,
    PeerReview,
    SelfReport,
    Casual,
}

impl Source {
    pub fn default_noise(self) -> f64 {
        match self {
            Source::StandardizedTest => 20.0,
            Source::PeerReview => 80.0,
            Source::SelfReport => 200.0,
            Source::Casual => 400.0,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Source::StandardizedTest => "standardized_test",
            Source::PeerReview => "peer_review",
            Source::SelfReport => "self_report",
            Source::Casual => "casual",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ObserveRequest {
    /// New raw score, 0-100
    pub score: f64,
    pub source: Source,
    /// Optional override of the source's default measurement noise
    pub measurement_noise: Option<f64>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ObserveResponse {
    pub skill: Skill,
    pub observation: Observation,
    pub kalman_gain: f64,
}
