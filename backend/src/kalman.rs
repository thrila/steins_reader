/// Recursive Bayesian (Kalman) update for a single scalar skill estimate.
///
/// State: x ~ N(mean, variance)
/// We treat each new score as a noisy measurement of the "true" skill level.

const MIN_VARIANCE: f64 = 1.5; // floor so confidence never collapses to zero
const MAX_VARIANCE: f64 = 900.0; // ceiling so uncertainty can't blow up
const REGRESSION_TARGET: f64 = 50.0;
const REGRESSION_RATE: f64 = 0.002; // gentle pull toward baseline per day idle

pub struct PredictResult {
    pub mean: f64,
    pub variance: f64,
}

/// Predict step: widen uncertainty with elapsed time (process noise),
/// and optionally let the mean drift slightly toward a neutral baseline
/// to model skill atrophy from disuse.
pub fn predict(mean: f64, variance: f64, process_noise: f64, dt_days: f64) -> PredictResult {
    let dt = dt_days.max(0.0);
    let widened = (variance + process_noise * dt).min(MAX_VARIANCE);
    let drifted = mean + REGRESSION_RATE * dt * (REGRESSION_TARGET - mean);
    PredictResult {
        mean: drifted.clamp(0.0, 100.0),
        variance: widened,
    }
}

pub struct UpdateResult {
    pub mean: f64,
    pub variance: f64,
    pub gain: f64,
}

/// Update step: blend prior estimate with a new observation.
/// `measurement_noise` (R) encodes how much we trust this particular score:
/// low R (e.g. a standardized test) pulls the estimate hard toward the new
/// score; high R (e.g. a casual guess) barely moves it.
pub fn update(
    prior_mean: f64,
    prior_variance: f64,
    score: f64,
    measurement_noise: f64,
) -> UpdateResult {
    let score = score.clamp(0.0, 100.0);
    let r = measurement_noise.max(1.0);
    let k = prior_variance / (prior_variance + r);
    let new_mean = (prior_mean + k * (score - prior_mean)).clamp(0.0, 100.0);
    let new_variance = ((1.0 - k) * prior_variance).clamp(MIN_VARIANCE, MAX_VARIANCE);
    UpdateResult {
        mean: new_mean,
        variance: new_variance,
        gain: k,
    }
}

/// Convenience: run predict immediately followed by update, given the
/// number of days since the skill was last touched.
pub fn predict_and_update(
    prior_mean: f64,
    prior_variance: f64,
    process_noise: f64,
    dt_days: f64,
    score: f64,
    measurement_noise: f64,
) -> UpdateResult {
    let p = predict(prior_mean, prior_variance, process_noise, dt_days);
    update(p.mean, p.variance, score, measurement_noise)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confident_test_score_dominates_uncertain_prior() {
        // brand new skill: high variance, score of 100 on a trusted test
        let r = predict_and_update(10.0, 400.0, 1.5, 0.0, 100.0, 20.0);
        assert!(
            r.mean > 75.0,
            "mean should jump close to the new score: {}",
            r.mean
        );
        assert!(
            r.variance < 400.0,
            "variance should shrink after observation"
        );
    }

    #[test]
    fn established_profile_resists_outlier() {
        // confident estimate (low variance) gets a single wildly different casual score
        let r = predict_and_update(80.0, 10.0, 1.5, 1.0, 10.0, 400.0);
        assert!(
            r.mean > 70.0,
            "single casual outlier shouldn't crater the mean: {}",
            r.mean
        );
    }

    #[test]
    fn variance_grows_with_time_since_last_update() {
        let short = predict(50.0, 50.0, 2.0, 1.0);
        let long = predict(50.0, 50.0, 2.0, 60.0);
        assert!(long.variance > short.variance);
    }
}
