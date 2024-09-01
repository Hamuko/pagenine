use crate::pushover::PushoverClientTrait;
use chrono::prelude::{DateTime, Utc};

#[derive(Default, Debug)]
pub struct State {
    pub thread: Option<Thread>,
    pub notified: i32,
}

impl State {
    pub fn new() -> Self {
        State {
            thread: None,
            notified: 0,
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Thread {
    pub page: i32,
    pub no: i32,
    pub sub: String,
    pub time: DateTime<Utc>,
    pub position: i32,
    pub page_length: i32,
}

impl Thread {
    /// Check if the Thread should be refreshed from the API.
    pub fn check_if_needs_refresh(&self) -> bool {
        let minutes_since_refresh = self.time_in_minutes();
        return match self.page {
            1 => minutes_since_refresh >= 15,
            2 | 3 => minutes_since_refresh >= 10,
            4 | 5 => minutes_since_refresh >= 7,
            6 => minutes_since_refresh >= 5,
            7 => minutes_since_refresh >= 3,
            8 if (self.position as f32 / self.page_length as f32) < 0.5 => {
                minutes_since_refresh >= 2
            }
            _ => true,
        };
    }

    /// Display a operating system notification about the thread.
    pub async fn send_pushover_notification(
        &self,
        pushover_client: &impl PushoverClientTrait,
    ) -> Result<(), ()> {
        let message = format!(">page {}", self.page);
        return pushover_client
            .send_notification(message, Some(&self.sub))
            .await;
    }

    /// Display a operating system notification about the thread.
    pub fn show_notification(&self) -> Result<(), ()> {
        let message = format!(">page {}", self.page);
        let notification_handle = notify_rust::Notification::new()
            .summary(message.as_str())
            .body(self.sub.as_str())
            .show();
        return match notification_handle {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        };
    }

    /// Calculate how many full minutes since the refresh.
    fn time_in_minutes(&self) -> i32 {
        let time_difference = chrono::offset::Utc::now() - self.time;
        let offset: f64 = time_difference.num_milliseconds() as f64 / 1000.0;
        let rounded_offset = offset.round() as i32;
        return rounded_offset / 60;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;
    use test_case::test_case;

    #[test]
    fn state_new() {
        let state = State::new();
        assert!(state.thread.is_none());
        assert_eq!(state.notified, 0);
    }

    #[test_case(1, 885, false; "under page 1 threshold")]
    #[test_case(1, 966, true; "over page 1 threshold")]
    #[test_case(2, 557, false; "under page 2 threshold")]
    #[test_case(3, 600, true; "over page 3 threshold")]
    #[test_case(4, 9, false; "under page 4 threshold")]
    #[test_case(5, 421, true; "over page 5 threshold")]
    #[test_case(6, 299, false; "under page 6 threshold")]
    #[test_case(6, 333, true; "over page 6 threshold")]
    #[test_case(7, 65, false; "under page 7 threshold")]
    #[test_case(7, 210, true; "over page 7 threshold")]
    #[test_case(9, 15, true; "always refreshable")]
    fn thread_check_if_needs_refresh(page: i32, seconds: i64, needs_refresh: bool) {
        let thread = Thread {
            page,
            no: 1,
            sub: String::new(),
            time: chrono::offset::Utc::now() - Duration::seconds(seconds),
            position: 1,
            page_length: 2,
        };
        assert_eq!(thread.check_if_needs_refresh(), needs_refresh);
    }

    #[test_case(6, 88, false; "under former threshold")]
    #[test_case(8, 130, true; "over former threshold")]
    #[test_case(10, 10, true; "latter always refreshable")]
    fn thread_check_if_needs_refresh_page_8(position: i32, seconds: i64, needs_refresh: bool) {
        let thread = Thread {
            page: 8,
            no: 1,
            sub: String::new(),
            time: chrono::offset::Utc::now() - Duration::seconds(seconds),
            position: position,
            page_length: 20,
        };
        assert_eq!(thread.check_if_needs_refresh(), needs_refresh);
    }

    #[test_case(276, 4; "under closest minute")]
    #[test_case(300, 5; "even minute")]
    #[test_case(305, 5; "over closest minute")]
    fn thread_time_since_closest_minute(seconds: i64, minutes: i32) {
        let thread = Thread {
            page: 1,
            no: 1,
            sub: String::new(),
            time: chrono::offset::Utc::now() - Duration::seconds(seconds),
            position: 1,
            page_length: 2,
        };
        assert_eq!(thread.time_in_minutes(), minutes);
    }
}
