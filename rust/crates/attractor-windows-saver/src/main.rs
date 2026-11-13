//! Entry point of the `.scr` executable.
//!
//! Windows starts the file with `/s`, `/c` or `/p`; a double click arrives as
//! an empty command line and opens the settings. The `--self-test` switches are
//! the only ones that work away from Windows, which keeps CI and a developer's
//! machine able to render the same pipeline.

// A screen saver is a windowed program: without this subsystem attribute
// Windows gives the console-subsystem binary a console window, which flashes
// on the desktop and shows up in the taskbar for the whole session. The
// `--self-test` output still reaches a pipe, so `cargo run` and CI capture it.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::ExitCode;

use attractor_windows_saver::{
    Command, CommandLine, SelfTestOutcome, SelfTestRequest, run_self_test,
};

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let command = match CommandLine::parse(&arguments) {
        Ok(line) => line.command,
        Err(error) => {
            eprintln!("error: {error}\n");
            eprint!("{}", CommandLine::USAGE);
            return ExitCode::from(2);
        }
    };

    match command {
        Command::Help => {
            print!("{}", CommandLine::USAGE);
            ExitCode::SUCCESS
        }
        Command::SelfTest(request) => self_test(request),
        other => run_shell(other),
    }
}

/// Render one offscreen frame and report it; the only mode that runs away from
/// Windows.
fn self_test(request: SelfTestRequest) -> ExitCode {
    match run_self_test(request) {
        Ok(SelfTestOutcome::Rendered(report)) => {
            println!(
                "self-test: {}x{}, {} frames, adapter '{}', backend {}, {} lit pixels",
                report.width,
                report.height,
                report.frames,
                report.adapter,
                report.backend,
                report.lit_pixels
            );
            if !report.rasterized {
                println!("self-test: the noop backend validated the pipelines only");
            }
            ExitCode::SUCCESS
        }
        Ok(SelfTestOutcome::Skipped { reason }) => {
            println!("self-test: skipped, no GPU adapter ({reason})");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

/// Run one of the Win32 modes.
fn run_shell(command: Command) -> ExitCode {
    #[cfg(not(windows))]
    {
        let _ = command;
        eprintln!(
            "error: the screen saver shell runs only on Windows; \
             use --self-test elsewhere"
        );
        ExitCode::from(1)
    }

    #[cfg(windows)]
    {
        let settings = load_settings();
        let result = match command {
            Command::Run => attractor_windows_saver::windows::run_saver(&settings),
            Command::Preview { parent } => {
                attractor_windows_saver::windows::run_preview(parent, &settings)
            }
            Command::Configure { owner } => {
                attractor_windows_saver::windows::run_configuration(owner)
            }
            Command::Help | Command::SelfTest(_) => unreachable!("handled by main"),
        };
        match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                log::error!("{error}");
                attractor_windows_saver::windows::show_error(&error.to_string());
                ExitCode::from(1)
            }
        }
    }
}

/// The saved settings, or the defaults when the file is missing or unreadable.
///
/// The saver always starts: a broken settings file must not leave the user
/// with a locked screen and no attractor.
#[cfg(windows)]
fn load_settings() -> attractor_windows_saver::Settings {
    use attractor_windows_saver::Settings;

    let Some(path) = Settings::environment_path() else {
        return Settings::default();
    };
    match Settings::load(&path) {
        Ok(settings) => settings,
        Err(error) => {
            log::warn!("cannot read {}: {error}", path.display());
            Settings::default()
        }
    }
}
