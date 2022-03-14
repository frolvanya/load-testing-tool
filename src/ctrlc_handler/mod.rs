use colored::Colorize;
use std::sync::{atomic::Ordering, Arc};

pub async fn ctrl_c_handler(data: Arc<crate::LoadTestingTool>) {
    tokio::signal::ctrl_c().await.unwrap();

    println!();

    crate::display::time::display_time();
    println!(
        "{}",
        format!(
            "{} {}",
            "Total Req: ".green(),
            format!("{}", data.total_requests.load(Ordering::SeqCst)).bold(),
        )
    );

    crate::display::time::display_time();
    println!(
        "{}",
        format!(
            "{} {}",
            "Failed Req: ".green(),
            format!("{}", data.failed_requests.load(Ordering::SeqCst)).bold(),
        )
    );

    crate::display::time::display_time();
    println!(
        "{}",
        format!(
            "{} {} {}",
            "Program worked for".green(),
            format!(
                "{:.02}",
                data.start_attack_time.elapsed().as_secs_f64() / 60.
            )
            .bold(),
            "min".green()
        )
    );

    crate::display::time::display_time();
    println!(
        "{}",
        format!(
            "{} {}",
            "Average RPS:".green(),
            format!(
                "{:.02}",
                data.total_requests.load(Ordering::SeqCst) as f64
                    / data.start_attack_time.elapsed().as_secs_f64()
            )
            .bold()
        )
    );
    std::process::exit(1);
}
