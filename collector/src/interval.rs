use chrono::{DateTime, Utc};

use crate::models::{LifecycleEvent, LifecycleType, StoredWindowEvent, TimeEvent, TimeEventKind};

pub fn build_time_events(events: &[StoredWindowEvent]) -> Vec<TimeEvent> {
    build_time_events_with_lifecycle(events, &[])
}

pub fn build_time_events_with_lifecycle(
    events: &[StoredWindowEvent],
    lifecycle_events: &[LifecycleEvent],
) -> Vec<TimeEvent> {
    let mut intervals = Vec::new();

    for (index, event) in events.iter().enumerate() {
        let next_window_at = events
            .get(index + 1)
            .filter(|next| next.session_id == event.session_id)
            .map(|next| next.event_ts);
        let next_lifecycle_cut = lifecycle_events
            .iter()
            .filter(|lifecycle| lifecycle.session_id == event.session_id)
            .filter(|lifecycle| {
                lifecycle.event_ts > event.event_ts
                    || (lifecycle.event_ts == event.event_ts
                        && lifecycle.lifecycle_type == LifecycleType::CollectorGap)
            })
            .filter(|lifecycle| {
                next_window_at
                    .map(|window_at| lifecycle.event_ts <= window_at)
                    .unwrap_or(true)
            })
            .filter(|lifecycle| cuts_active_interval(&lifecycle.lifecycle_type))
            .map(|lifecycle| lifecycle.event_ts)
            .min();
        let ended_at = earliest(next_window_at, next_lifecycle_cut);
        let duration_seconds = duration_seconds(event.event_ts, ended_at);

        intervals.push(TimeEvent {
            id: format!("raw-{}", event.raw_event_id),
            app: event.process_name.clone(),
            title: event.window_title.clone().unwrap_or_default(),
            kind: TimeEventKind::ActiveWindow,
            status: None,
            session_id: event.session_id.clone(),
            started_at: event.event_ts,
            ended_at,
            duration_seconds,
        });
    }

    for lifecycle in lifecycle_events {
        let Some((title, end_type)) = lifecycle_interval_pair(&lifecycle.lifecycle_type) else {
            continue;
        };
        let ended_at = lifecycle_events
            .iter()
            .filter(|candidate| candidate.session_id == lifecycle.session_id)
            .filter(|candidate| candidate.event_ts > lifecycle.event_ts)
            .filter(|candidate| candidate.lifecycle_type == end_type)
            .map(|candidate| candidate.event_ts)
            .min()
            .or_else(|| {
                events
                    .iter()
                    .filter(|event| event.session_id == lifecycle.session_id)
                    .filter(|event| event.event_ts > lifecycle.event_ts)
                    .map(|event| event.event_ts)
                    .min()
            });

        intervals.push(TimeEvent {
            id: format!("lifecycle-{}", lifecycle.raw_event_id),
            app: "System".to_string(),
            title: title.to_string(),
            kind: TimeEventKind::Lifecycle,
            status: Some(lifecycle.lifecycle_type.as_str().to_string()),
            session_id: lifecycle.session_id.clone(),
            started_at: lifecycle.event_ts,
            ended_at,
            duration_seconds: duration_seconds(lifecycle.event_ts, ended_at),
        });
    }

    intervals.sort_by(|left, right| {
        left.started_at
            .cmp(&right.started_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    intervals
}

fn earliest(left: Option<DateTime<Utc>>, right: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn duration_seconds(started_at: DateTime<Utc>, ended_at: Option<DateTime<Utc>>) -> Option<i64> {
    ended_at.map(|end| end.signed_duration_since(started_at).num_seconds().max(0))
}

fn cuts_active_interval(lifecycle_type: &LifecycleType) -> bool {
    matches!(
        lifecycle_type,
        LifecycleType::WindowsLock
            | LifecycleType::PowerSuspend
            | LifecycleType::IdleStart
            | LifecycleType::CaptureUnavailable
            | LifecycleType::CollectorGap
            | LifecycleType::SessionStop
            | LifecycleType::SessionDisconnect
    )
}

fn lifecycle_interval_pair(
    lifecycle_type: &LifecycleType,
) -> Option<(&'static str, LifecycleType)> {
    match lifecycle_type {
        LifecycleType::WindowsLock => Some(("Locked", LifecycleType::WindowsUnlock)),
        LifecycleType::PowerSuspend => Some(("Suspended", LifecycleType::PowerResume)),
        LifecycleType::IdleStart => Some(("Idle", LifecycleType::IdleEnd)),
        LifecycleType::SessionDisconnect => {
            Some(("Session disconnected", LifecycleType::SessionReconnect))
        }
        _ => None,
    }
}
