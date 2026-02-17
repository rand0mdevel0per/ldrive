use crate::types::CreditRecord;

const BASE_RATE: f64 = 1.0;
const UPTIME_THRESHOLD: f64 = 0.8;

pub fn calculate_credits(
    node_id: String,
    storage_gb: f64,
    uptime_hours: f64,
    epoch_hours: f64,
    challenge_pass_rate: f64,
) -> CreditRecord {
    let storage_factor = (1.0 + storage_gb).ln() / (1.0_f64 + 1000.0).ln();

    let uptime_factor = if uptime_hours / epoch_hours < UPTIME_THRESHOLD {
        0.0
    } else {
        uptime_hours / epoch_hours
    };

    let response_factor = challenge_pass_rate.powi(3);

    let credits_earned = BASE_RATE * storage_factor * uptime_factor * response_factor;

    CreditRecord {
        node_id,
        storage_gb,
        uptime_hours,
        challenge_pass_rate,
        credits_earned,
    }
}
