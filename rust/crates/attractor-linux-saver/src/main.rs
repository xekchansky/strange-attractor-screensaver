//! Entry point of the Linux screen saver.
//!
//! xscreensaver starts the binary with `-root` or `-window-id N`; a desktop
//! launcher or an idle hook starts it with `--fullscreen`. `--self-test` is
//! the only mode that works away from Linux, which keeps CI and a developer's
//! machine able to render the same pipeline.

use std::process::ExitCode;

#[cfg(target_os = "linux")]
use attractor_linux_saver::Settings;
use attractor_linux_saver::{
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

/// Render one offscreen frame and report it; the only mode that runs away
/// from Linux.
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

/// Run the xscreensaver hack or the fullscreen runner.
fn run_shell(command: Command) -> ExitCode {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = command;
        eprintln!(
            "error: the screen saver shell runs only on Linux; \
             use --self-test elsewhere"
        );
        ExitCode::from(1)
    }

    #[cfg(target_os = "linux")]
    {
        let settings = load_settings();
        let result = match command {
            Command::Fullscreen { frames } => attractor_linux_saver::runner::run(&settings, frames),
            Command::Hack { window, frames } => {
                attractor_linux_saver::x11::run_hack(window, &settings, frames)
            }
            Command::Help | Command::SelfTest(_) => unreachable!("handled by main"),
        };
        match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                log::error!("{error}");
                eprintln!("error: {error}");
                ExitCode::from(1)
            }
        }
    }
}

/// The saved settings, or the defaults when the file is missing or unreadable.
///
/// The saver always starts: a broken settings file must not leave the user
/// with a locked screen and no attractor.
#[cfg(target_os = "linux")]
fn load_settings() -> Settings {
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
