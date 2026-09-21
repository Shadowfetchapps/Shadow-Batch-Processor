mod app;
mod ui;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Shadow Batch Processor — bulk media pipelines\n\n\
             Usage:\n\
             \tshadow-batch-processor [OPTIONS] [FILE...]\n\n\
             Options:\n\
             \t--help, -h      Show this help\n\
             \t--version, -V   Show version\n"
        );
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("shadow-batch-processor {}", shadow_batch::paths::APP_VERSION);
        return ExitCode::SUCCESS;
    }
    app::run()
}
