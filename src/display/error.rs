use colored::Colorize;

pub fn display_error(text: String, error_mode: bool) {
    if !error_mode {
        super::time::display_time();
        println!("{}", text.red().bold());
    }
}
