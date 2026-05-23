use crate::models::{StoredWindowEvent, TimeEvent};

pub fn build_time_events(events: &[StoredWindowEvent]) -> Vec<TimeEvent> {
    events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let ended_at = events.get(index + 1).map(|next| next.event_ts);
            let duration_seconds = ended_at.map(|end| {
                end.signed_duration_since(event.event_ts)
                    .num_seconds()
                    .max(0)
            });

            TimeEvent {
                id: format!("raw-{}", event.raw_event_id),
                app: event.process_name.clone(),
                title: event.window_title.clone().unwrap_or_default(),
                started_at: event.event_ts,
                ended_at,
                duration_seconds,
            }
        })
        .collect()
}
