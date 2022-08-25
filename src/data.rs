use chrono::prelude::{DateTime, Utc};

#[derive(Default, Debug)]
pub struct Thread {
    pub page: i32,
    pub no: i32,
    pub sub: String,
    pub time: DateTime<Utc>,
}

impl Thread {
    /// Check if the Thread should be refreshed from the API.
    pub fn check_if_needs_refresh(self: &Self) -> bool {
        let minutes_since_refresh = self.time_since_closest_minute();
        return match self.page {
            1 => minutes_since_refresh >= 15,
            2 | 3 => minutes_since_refresh >= 10,
            4 | 5 => minutes_since_refresh >= 7,
            6 => minutes_since_refresh >= 5,
            7 => minutes_since_refresh >= 3,
            _ => true,
        };
    }

    /// Display a operating system notification about the thread.
    pub fn show_notification(
        self: &Self,
    ) -> Result<notify_rust::NotificationHandle, notify_rust::error::Error> {
        let message = format!(">page {}", self.page);
        return notify_rust::Notification::new()
            .summary(message.as_str())
            .body(self.sub.as_str())
            .show();
    }

    /// Calculate how many minutes old the
    fn time_since_closest_minute(self: &Self) -> i32 {
        let time_difference = chrono::offset::Utc::now() - self.time;
        let offset: f64 = time_difference.num_milliseconds() as f64 / 1000.0;
        let rounded_offset = offset.round() as i32;
        return rounded_offset / 60;
    }
}
