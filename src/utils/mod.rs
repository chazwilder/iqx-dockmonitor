use chrono::Duration;


/// Formats a duration into a human-readable string
///
/// # Arguments
///
/// * `duration` - The duration to format
///
/// # Returns
///
/// A formatted duration string
pub fn format_duration(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}