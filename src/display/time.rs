use chrono::{Local, Timelike};
use colored::Colorize;

pub fn display_time() {
    let now = Local::now().time();
    let time = format!(
        "[{:02}:{:02}:{:02}] ",
        now.hour(),
        now.minute(),
        now.second()
    );

    print!("{} ", time.blue());
}
