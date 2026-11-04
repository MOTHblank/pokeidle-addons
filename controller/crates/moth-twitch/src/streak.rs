//! Streak eligibility is tied to Twitch's current broadcast, not calendar
//! midnight or successful telemetry POSTs. Unparseable timestamps stay unknown.

/// Twitch timestamps are normally RFC3339 UTC. Reject offsets rather than
/// comparing strings that would incorrectly order different time zones.
pub(crate) fn utc_key(value: &str) -> Option<(&str, u32)> {
    let content=value.strip_suffix('Z')?;
    let (seconds, fractional)=content.split_once('.')
        .map(|(a,b)|(a,b)).unwrap_or((content,""));
    if seconds.len()!=19 || !seconds.as_bytes().iter().enumerate()
        .all(|(i,b)| if matches!(i,4|7){*b==b'-'}else if i==10{*b==b'T'}
             else if matches!(i,13|16){*b==b':'}else{b.is_ascii_digit()}) {
        return None;
    }
    if fractional.len()>9 || !fractional.bytes().all(|b|b.is_ascii_digit()) {
        return None;
    }
    let mut nanos=fractional.to_owned();
    while nanos.len()<9 {nanos.push('0');}
    Some((seconds,nanos.parse().ok()?))
}

pub(crate) fn milestone_for_current_broadcast(
    started: &str, achieved: &str,
) -> Option<bool> {
    let start=utc_key(started)?;
    let achievement=utc_key(achieved)?;
    Some(achievement>start)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_a_milestone_after_broadcast_start_counts() {
        assert_eq!(milestone_for_current_broadcast(
            "2026-11-04T20:00:00Z","2026-11-04T20:04:00.123Z"),Some(true));
        assert_eq!(milestone_for_current_broadcast(
            "2026-11-04T20:00:00Z","2026-11-03T23:59:59Z"),Some(false));
        assert_eq!(milestone_for_current_broadcast(
            "2026-11-04T20:00:00Z","2026-11-04T20:00:00Z"),Some(false));
        assert_eq!(milestone_for_current_broadcast(
            "2026-11-04T20:00:00+03:00","2026-11-04T20:04:00Z"),None);
    }
}
