//! `attractor-cli`: the cross-platform capture demo.

use std::process::ExitCode;

use attractor_cli::{CaptureReport, CaptureRequest, CommandLineArguments, run};

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match CommandLineArguments::parse(&arguments) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("error: {error}\n");
            print_usage(true);
            return ExitCode::from(2);
        }
    };
    if parsed.show_help {
        print_usage(false);
        return ExitCode::SUCCESS;
    }

    match run(&parsed) {
        Ok(report) => {
            print_report(&report, &parsed);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            let code = error.exit_code();
            if code == 2 {
                eprintln!();
                print_usage(true);
            }
            ExitCode::from(code)
        }
    }
}

fn print_report(report: &CaptureReport, arguments: &CommandLineArguments) {
    match &arguments.capture {
        Some(CaptureRequest::Screenshot(path)) => println!(
            "capture: {}x{}, {} frames simulated, {} lit pixels, wrote {}",
            report.width,
            report.height,
            report.frames_simulated,
            report.lit_pixels,
            path.display()
        ),
        Some(CaptureRequest::Record {
            output_directory, ..
        }) => println!(
            "capture: {}x{}, {} frames written to {} ({} lit pixels in the last frame)",
            report.width,
            report.height,
            report.frames_written,
            output_directory.display(),
            report.lit_pixels
        ),
        None => {}
    }
}

fn print_usage(to_stderr: bool) {
    let mut text = String::from(CommandLineArguments::USAGE);
    if let Ok(catalog) = attractor_core::catalog::bundled() {
        text.push_str("\nAvailable attractors:\n");
        for descriptor in catalog {
            let parameters = descriptor
                .parameters
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push_str(&format!(
                "  {}    parameters: {parameters}\n",
                descriptor.id
            ));
        }
    }
    if to_stderr {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
}
